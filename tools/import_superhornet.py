"""Adapt the pinned GPL FlightGear F-18F/G exterior to the E and G variants.

Original package: F-GTUX, 2026-03-13, geometry by Petar Jedvaj.
No FlightGear program or unrepresentative source instrument panel is executed.
"""
from pathlib import Path
import argparse, gzip, hashlib, json, math, shutil, struct, zlib
import import_flightgear as fg

ROOT=Path(__file__).resolve().parents[1]
SX=13.7/(2*6.58995)
SY=4.88/(5.13838+0.01097)
SZ=18.3/(9.18164+9.08501)

def point(p):
    return (-p[2]*SX,(p[1]-2.20)*SY,p[0]*SZ+0.15)

def canopy_e(p):
    # The E retains the same front windscreen. Its single-seat canopy ends
    # 1.31 m before the F/G's rear hinge; the vacated bay becomes the spine.
    x,y,z=p
    if x>-4.754:x=-4.754+(x+4.754)*(2.35/3.662)
    return (x,y,z)

def atlas_uv(u,v,rect,size):
    # AC3D UVs start at the bottom; decoded QOI rows start at the top.
    x,y,w,h=rect;aw,ah=size
    return ((x+.5+u*(w-1))/aw,(y+.5+(1-v)*(h-1))/ah)

def generate(source,out):
    from PIL import Image
    source=source.resolve();out.mkdir(parents=True,exist_ok=True)
    main=source/'Models/F-18F.ac'
    objects=list(fg.flatten(fg.read_ac(main)))
    used={main,source/'Models/Diffuse.jpg',source/'Models/common.xml',
          source/'F-18F-yasim-set.xml',source/'EA-18G-yasim-set.xml',
          source/'Systems/views.xml',source/'Models/PjedvajAutorization.png'}
    skins={}
    for variant,file,livery in [('f18e','superhornet-skin.qoi','Liveries/VFA22.png'),('ea18g','growler-skin.qoi','LiveriesG/VAQ129.png')]:
        used.add(source/'Models'/livery)
        used.add((source/'Models'/livery).with_suffix('.xml'))
        skin=fg.styled_texture(Image.open(source/'Models'/livery))
        tiles={'body':(0,0,*skin.size)}
        if variant=='ea18g':
            path=source/'Models/Stores/ALQ/alq.png';used.add(path)
            pod=fg.styled_texture(Image.open(path));atlas=Image.new('RGB',(skin.width,skin.height+pod.height),(145,153,158))
            atlas.paste(skin,(0,0));atlas.paste(pod,(0,skin.height))
            tiles['alq']=(0,skin.height,*pod.size);skin=atlas
        skins[variant]=(skin.size,tiles)
        (out/file).write_bytes(fg.qoi(skin))
    report={}
    for variant in ('f18e','ea18g'):
        parts={name:[] for name in ('body','canopy','frame','cockpit','brake',
            'stab0','stab1','flap0','flap1','rudder0','rudder1',
            'gear0','gear1','gear2','aileron0','aileron1',
            'slat_inner0','slat_inner1','slat_outer0','slat_outer1')}
        inventory=[]
        size,tiles=skins[variant]
        def emit(o,vs,part,flat=None,remap=False,tile='body'):
            vs=[point(canopy_e(v) if remap else v) for v in vs]
            if o.name.startswith(('Pylon1','Pylon2','Pylon3')):
                side=1 if sum(v[0] for v in vs)>0 else -1
                x,y,z=(2.384,1.473,1.18) if o.name.startswith('Pylon1') else (3.314,1.433,1.65) if o.name.startswith('Pylon2') else (4.288,1.417,1.90)
                pivot=point((z,y,-side*x));angle=-side*math.radians(3.5)
                def yaw(p):
                    dx,dz=p[0]-pivot[0],p[2]-pivot[2]
                    return (pivot[0]+dx*math.cos(angle)+dz*math.sin(angle),p[1],pivot[2]-dx*math.sin(angle)+dz*math.cos(angle))
                vs=[yaw(p) for p in vs]
            arr=parts[part]
            entry={'name':o.name,'part':part,'bounds':fg.bounds(vs),'vertex_start':len(arr)//11}
            inventory.append(entry)
            for material,refs in o.faces:
                for ids in fg.triangulate([vs[r[0]] for r in refs]):
                    tri=[refs[i] for i in ids]
                    a,b,c=[vs[r[0]] for r in tri]
                    n=fg.unit(fg.cross(fg.sub(b,a),fg.sub(c,a)))
                    if not all(math.isfinite(x) for x in n):raise ValueError(o.name)
                    textured=flat is None and bool(o.texture)
                    colour=flat or ((1.,1.,1.) if textured else o.materials[material].rgb)
                    for index,u,v in tri:
                        uv=atlas_uv(u,v,tiles[tile],size) if textured else (-1.,-1.)
                        arr.extend((*vs[index],*n,*colour,*uv))
            entry['vertex_count']=len(arr)//11-entry['vertex_start']
        def wheel(o,vs,part):
            # Preserve measured axle, radius and width with a compact tyre,
            # rather than thousands of triangles on hidden sidewalls.
            lo,hi=fg.bounds(vs);cx=(lo[0]+hi[0])*.5;cy=(lo[1]+hi[1])*.5
            radius=(hi[1]-lo[1])*.5
            centres=[(-.225,.172),(.225,.172)] if o.name=='NoseWheels' else [((lo[2]+hi[2])*.5,hi[2]-lo[2])]
            for cz,width in centres:
                profile=[(-.5,.72),(-.45,.94),(-.30,1.0),(.30,1.0),(.45,.94),(.5,.72)]
                vertices=[];faces=[]
                for a,r in profile:
                    for i in range(24):
                        phi=i*math.tau/24
                        vertices.append((cx+radius*r*math.cos(phi),cy+radius*r*math.sin(phi),cz+a*width))
                for j in range(len(profile)-1):
                    for i in range(24):
                        ids=[j*24+i,j*24+(i+1)%24,(j+1)*24+(i+1)%24,(j+1)*24+i]
                        faces.append((0,[(k,0.,0.) for k in ids]))
                for end in [0,len(profile)-1]:
                    centre=len(vertices);vertices.append((cx,cy,cz+profile[end][0]*width))
                    for i in range(24):faces.append((0,[(k,0.,0.) for k in [centre,end*24+i,end*24+(i+1)%24]]))
                compact=fg.Object(name=o.name,faces=faces,materials=o.materials)
                emit(compact,vertices,part,(.10,.11,.12))
        for o,vs in objects:
            n=o.name
            # Source stores are presentation loads, not part of the aircraft.
            if n.startswith(('AIM-','GBU-','Tank','Seat')) or n in ('Furniture','Cockpit','AirbrakeBox','AirbrakeTBox'):continue
            if n in ('SlatsInnerLG','SlatsInnerRG','AntennasG','LeftFence','RightFence','SATCOM') and variant!='ea18g':continue
            if n in ('SlatsInnerL','SlatsInnerR','Antennas') and variant=='ea18g':continue
            if variant=='ea18g' and n.startswith('PylonWingTip'):continue
            side=0 if sum(v[2] for v in vs)>0 else 1
            part='body';flat=None
            if n.startswith('CanopyGlass'):part='canopy';flat=(.35,.48,.50)
            elif n in ('Canopy','CanopyFront'):part='frame';flat=(.07,.08,.09)
            elif n.startswith('Elevator'):part=f'stab{side}'
            elif n.startswith('Flaps'):part=f'flap{side}'
            elif n.startswith('Rudder'):part=f'rudder{side}'
            elif n.startswith('Elerons'):part=f'aileron{side}'
            elif n.startswith('SlatsInner'):part=f'slat_inner{side}'
            elif n.startswith('SlatsOuter'):part=f'slat_outer{side}'
            elif n.startswith('GearDoors'):part=f'gear{side+1}'
            elif n.startswith('NoseDoors'):part='gear0'
            elif any(k in n for k in ('Strut','Wheel')) or n=='CatobarHook':
                part=f'gear{side+1}' if n.endswith(('L','R','LR')) else 'gear0'
            if n=='F414Nozzles':flat=(.29,.31,.32)
            if n in ('Inlets','FrontGearBox','LandingGearBox'):flat=(.08,.10,.11)
            # The original artist included a dorsal brake which the real
            # Super Hornet lacks. Keep its roof fixed and erase its painted seam.
            if n.startswith('Airbrake'):flat=(.58,.62,.65)
            if 'Wheel' in n:
                wheel(o,vs,part);continue
            emit(o,vs,part,flat,variant=='f18e' and n in ('Canopy','CanopyGlass'))
        if variant=='ea18g':
            import xml.etree.ElementTree as ET
            root=ET.parse(source/'Models/common.xml').getroot()
            for model in root.findall('model'):
                path=model.findtext('path','')
                if 'ALQ218' not in path:continue
                xml=source/path.split('Aircraft/F-18F/')[1]
                used.add(xml)
                # Retain the same measured source transform used by FlightGear.
                r,t=fg.xml_pose(model.find('offsets'))
                ac=xml.parent/ET.parse(xml).getroot().findtext('path')
                used.add(ac)
                for o,vs in fg.flatten(fg.read_ac(ac)):
                    vs=[fg.fg_ac(fg.add(fg.mv(r,fg.ac_fg(v)),t)) for v in vs]
                    emit(o,vs,'body',tile='alq')
        data=bytearray(b'VRM1'+struct.pack('<I',len(parts)))
        for name,arr in parts.items():
            b=name.encode();data.extend(struct.pack('<I',len(b))+b+struct.pack('<I',len(arr)))
            data.extend(struct.pack('<'+'f'*len(arr),*arr))
        data.extend(struct.pack('<I',0))
        (out/(variant+'.vrm')).write_bytes(data)
        (out/(variant+'.vrz')).write_bytes(b'VRZ1'+struct.pack('<I',len(data))+zlib.compress(data,9))
        report[variant]={'parts':{k:len(v)//33 for k,v in parts.items()},'objects':inventory,
                         'transform':{'right':SX,'up':SY,'aft':SZ,'source_up_datum':2.20,'aft_offset':.15},
                         'texture':{'size':size,'tiles':tiles,'row_origin':'top'}}
        print(variant,report[variant]['parts'],len(data),flush=True)
    (out/'superhornet-geometry.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8',newline='\n')
    manifest=[]
    for p in sorted(used):
        rel=p.relative_to(source);dest=out/'source/superhornet'/rel;dest.parent.mkdir(parents=True,exist_ok=True)
        data=p.read_bytes();manifest.append({'path':rel.as_posix(),'sha256':hashlib.sha256(data).hexdigest()})
        if p.suffix in ('.ac','.xml'):Path(str(dest)+'.gz').write_bytes(gzip.compress(data,mtime=0))
        else:shutil.copyfile(p,dest)
    (out/'superhornet-sources.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf-8',newline='\n')

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source-root',type=Path,required=True)
    p.add_argument('--output',type=Path,default=ROOT/'assets/models')
    a=p.parse_args();generate(a.source_root,a.output)
