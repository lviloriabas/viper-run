"""Bake GPL FlightGear paint onto the existing procedural F-16/MiG-29 atlas.

Only paint is transferred. Source geometry is a depth-tested projection donor,
not a replacement for the game's articulated aircraft or cockpit.
"""
from pathlib import Path
import argparse,gzip,hashlib,json,shutil
import numpy as np
from PIL import Image,ImageDraw
import import_flightgear as fg

ROOT=Path(__file__).resolve().parents[1]
SIZE=(2048,1024)
LAYOUT={'f16':{'z':(-8.,7.6),'x':5.,'y':(3.9,-.7)},
        'mig29':{'z':(-9.,8.7),'x':6.,'y':(3.6,-1.6)}}
SOURCES={
 'f16':{'repo':'https://github.com/NikolaiVChr/f16','revision':'ef4f192f06243cf4061c2be794511db0576ac2f9','licence':'GPL-2.0-or-later',
        'files':['LICENSE','copyright.txt','authors.txt','Models/f16.ac','Models/f16.png','Models/Liveries/Block50/91-0352.png','Models/Liveries/Block50/91-0352.xml']},
 'mig29':{'repo':'https://github.com/Mercenary-Mercury/MiG-29_9-12','revision':'d4a299bf88412f579b2067874989a8088b2f6d66','licence':'GPL-3.0',
          'files':['LICENSE','ReadMeEng.txt','Models/MiG-29.ac','Models/MiG-29_1.png','Models/MiG-29_2.png']}}

def source_image(path,jet):
    image=np.array(Image.open(path).convert('RGBA'))
    if jet=='f16':
        # A connected white canvas is not skin; isolated white insignia stay.
        blank=Image.fromarray(np.where((image[:,:,:3]>248).all(axis=2),255,0).astype(np.uint8)).copy()
        for corner in [(0,0),(blank.width-1,0),(0,blank.height-1),(blank.width-1,blank.height-1)]:
            if blank.getpixel(corner)==255:ImageDraw.floodfill(blank,corner,128)
        image[np.asarray(blank)==128,3]=0
    elif Path(path).name=='MiG-29_2.png':
        # Measured wheel-hub tile in the original 2048 atlas. Some donor caps
        # share its default UVs; it is not part of the grey-green airframe paint.
        image[1970:,:800,3]=0
    return image

def point(jet,p):
    x,y,z=p
    if jet=='f16':
        sz=14.18/(5.89448+7.66127)
        return (-z*(4.65/4.63011),y*((3.64-.27)/3.24977)+.27,(x+7.66127)*sz-7.09)
    return (-z*(5.68/5.72893),y*((3.176-.30)/2.634808)+.30,x+.34)

def projection(jet,view,p):
    a=LAYOUT[jet];x,y,z=p
    u=(z-a['z'][0])/(a['z'][1]-a['z'][0])
    v=(a['y'][0]-y)/(a['y'][0]-a['y'][1]) if view>=2 else (x+a['x'])/(2*a['x'])
    depth=(y,-y,x,-x)[view]
    return u,v,depth

def paint_objects(jet,source):
    name='f16.ac' if jet=='f16' else 'MiG-29.ac'
    for ob,vs in fg.flatten(fg.read_ac(source/'Models'/name)):
        n=ob.name.lower()
        if jet=='f16':
            if ob.texture!='f16.png' or any(s in n for s in ('fan','tire','strut','station','gear','door','flame','turbine','canopy','hook','box','rnlaf')):continue
            texture='Models/Liveries/Block50/91-0352.png'
        else:
            skin=('aileron','airbrake','eng_cover','fin','flap','lerx','louver','lower_pod','radome','rudder','stab','upperintakebox','wing','engtun','fuselage','upper_pod')
            if ob.texture not in ('MiG-29_1.png','MiG-29_2.png') or not n.startswith(skin) or n.startswith(('fuselage.piece','stab_root')):continue
            texture='Models/'+ob.texture
        yield ob,[point(jet,p) for p in vs],texture

def raster_triangle(rgb,depth,points,uv,image):
    """Sample source UV barycentrically; nearest visible skin wins each texel."""
    h,w=depth.shape;p=np.asarray(points,dtype=np.float64)
    x=p[:,0]*w-.5;y=p[:,1]*h-.5
    x0=max(0,int(np.floor(x.min())));x1=min(w-1,int(np.ceil(x.max())))
    y0=max(0,int(np.floor(y.min())));y1=min(h-1,int(np.ceil(y.max())))
    den=(y[1]-y[2])*(x[0]-x[2])+(x[2]-x[1])*(y[0]-y[2])
    if x1<x0 or y1<y0 or abs(den)<1e-9:return
    xx,yy=np.meshgrid(np.arange(x0,x1+1),np.arange(y0,y1+1))
    a=((y[1]-y[2])*(xx-x[2])+(x[2]-x[1])*(yy-y[2]))/den
    b=((y[2]-y[0])*(xx-x[2])+(x[0]-x[2])*(yy-y[2]))/den;c=1-a-b
    z=a*p[0,2]+b*p[1,2]+c*p[2,2]
    old=depth[y0:y1+1,x0:x1+1]
    mask=(a>=-1e-8)&(b>=-1e-8)&(c>=-1e-8)&(z>=old)
    if not mask.any():return
    uv=np.asarray(uv);u=a*uv[0,0]+b*uv[1,0]+c*uv[2,0]
    v=1-(a*uv[0,1]+b*uv[1,1]+c*uv[2,1])
    ih,iw=image.shape[:2];tx=np.clip(np.rint(u*(iw-1)),0,iw-1).astype(np.int32);ty=np.clip(np.rint(v*(ih-1)),0,ih-1).astype(np.int32)
    if image.shape[2]==4:mask &= image[ty,tx,3]>127
    tile=rgb[y0:y1+1,x0:x1+1];tile[mask]=image[ty[mask],tx[mask],:3];old[mask]=z[mask]

def pad(rgb,valid,steps=32):
    # Extend the nearest edge paint into small shape-fit differences, one view
    # at a time. Paint never leaks between the four atlas rectangles.
    for _ in range(steps):
        previous=valid.copy();changed=False
        for dy,dx in [(0,1),(0,-1),(1,0),(-1,0)]:
            src=(slice(max(dy,0),valid.shape[0]+min(dy,0)),slice(max(dx,0),valid.shape[1]+min(dx,0)))
            dst=(slice(max(-dy,0),valid.shape[0]-max(dy,0)),slice(max(-dx,0),valid.shape[1]-max(dx,0)))
            take=~valid[dst]&previous[src]
            if take.any():rgb[dst][take]=rgb[src][take];valid[dst][take]=True;changed=True
        if not changed:break

def generate(source,out,preview=None):
    out.mkdir(parents=True,exist_ok=True);report={}
    for jet in ('f16','mig29'):
        folder=source/jet
        if jet=='mig29' and not (folder/'Models/MiG-29.ac').exists():folder=source/'mig29-mercury'
        objects=list(paint_objects(jet,folder));images={}
        for _,_,texture in objects:
            if texture not in images:
                images[texture]=source_image(folder/texture,jet)
        triangles=[]
        for ob,vs,texture in objects:
            for _,refs in ob.faces:
                for ids in fg.triangulate([vs[r[0]] for r in refs]):
                    ps=[vs[refs[i][0]] for i in ids]
                    normal=fg.unit(fg.cross(fg.sub(ps[1],ps[0]),fg.sub(ps[2],ps[0])))
                    triangles.append((ps,normal,[refs[i][1:] for i in ids],images[texture]))
        atlas=Image.new('RGB',SIZE);counts=[]
        for view in range(4):
            w,h=SIZE[0]//2,SIZE[1]//2
            rgb=np.empty((h,w,3),dtype=np.uint8);rgb[:]=(153,159,164) if jet=='f16' else (148,157,157)
            depth=np.full((h,w),-np.inf)
            for ps,n,uv,image in triangles:
                if (n[1],-n[1],n[0],-n[0])[view]<.08:continue
                raster_triangle(rgb,depth,[projection(jet,view,p) for p in ps],uv,image)
            valid=np.isfinite(depth);counts.append(int(valid.sum()));pad(rgb,valid)
            tile=Image.fromarray(rgb)
            atlas.paste(tile,((view//2)*w,(view%2)*h))
        # No speckle, noise or dithering; keep fine source panel lines and legends.
        atlas=atlas.quantize(colors=96,method=Image.Quantize.MEDIANCUT,dither=Image.Dither.NONE).convert('RGB')
        data=fg.qoi(atlas);(out/(jet+'-skin.qoi')).write_bytes(data)
        if preview:preview.mkdir(parents=True,exist_ok=True);atlas.save(preview/(jet+'-atlas.png'))
        origin=point(jet,(0.,0.,0.));scales=[point(jet,p) for p in [(0,0,-1),(0,1,0),(1,0,0)]]
        transform={'axes':['-z','y','x'],'scale':[scales[i][i]-origin[i] for i in range(3)],'offset':origin,'purpose':'paint fit only'}
        row={**SOURCES[jet],'source_transform':transform,'atlas':{'size':SIZE,'layout':LAYOUT[jet],'order':['top','bottom','right','left'],'row_origin':'top'},
             'source_triangles':len(triangles),'covered_pixels':counts,'qoi_bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'inputs':[]}
        for name in SOURCES[jet]['files']:
            path=folder/name;raw=path.read_bytes();dest=out/'source/liveries'/jet/name;dest.parent.mkdir(parents=True,exist_ok=True)
            row['inputs'].append({'path':name,'sha256':hashlib.sha256(raw).hexdigest()})
            if path.suffix=='.ac':Path(str(dest)+'.gz').write_bytes(gzip.compress(raw,mtime=0))
            else:shutil.copyfile(path,dest)
        report[jet]=row;print(jet,len(triangles),'source triangles',counts,len(data),'QOI bytes',flush=True)
    (out/'livery-sources.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8',newline='\n')

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--source-root',type=Path,required=True)
    p.add_argument('--output',type=Path,default=ROOT/'assets/models');p.add_argument('--preview',type=Path)
    a=p.parse_args();generate(a.source_root,a.output,a.preview)
