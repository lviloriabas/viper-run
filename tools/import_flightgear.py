#!/usr/bin/env python3
"""Adapt pinned FlightGear AC3D sources without projecting images onto geometry."""
from __future__ import annotations
import argparse
from dataclasses import dataclass, field
import gzip, hashlib, json, math, re, shlex, struct, zlib
from pathlib import Path
import xml.etree.ElementTree as ET
ROOT = Path(__file__).resolve().parents[1]
IDENTITY = ((1.,0.,0.),(0.,1.,0.),(0.,0.,1.))
def add(a,b): return tuple(x+y for x,y in zip(a,b))
def sub(a,b): return tuple(x-y for x,y in zip(a,b))
def mul(a,s): return tuple(x*s for x in a)
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def cross(a,b): return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def unit(a):
    n=math.sqrt(dot(a,a))
    return mul(a,1/n) if n>1e-12 else (0.,1.,0.)
def mv(m,v): return tuple(dot(row,v) for row in m)
def mm(a,b): return tuple(tuple(sum(a[i][k]*b[k][j] for k in range(3)) for j in range(3)) for i in range(3))
@dataclass
class Material:
    name:str
    rgb:tuple
    alpha:float
@dataclass
class Object:
    name:str=""
    loc:tuple=(0.,0.,0.)
    rot:tuple=IDENTITY
    texture:str=""
    verts:list=field(default_factory=list)
    faces:list=field(default_factory=list)
    children:list=field(default_factory=list)
    materials:list=field(default_factory=list)
    path:Path|None=None

def read_ac(path):
    path=Path(path)
    raw=gzip.decompress(path.read_bytes()) if path.suffix==".gz" else path.read_bytes()
    lines=raw.decode("latin-1").splitlines()
    materials=[]; i=1
    while not lines[i].startswith("OBJECT"):
        t=shlex.split(lines[i])
        if t and t[0]=="MATERIAL":
            c=t.index("rgb")+1
            materials.append(Material(t[1],tuple(map(float,t[c:c+3])),1-float(t[t.index("trans")+1])))
        i+=1
    def obj():
        nonlocal i
        assert lines[i].startswith("OBJECT"),(path,i,lines[i])
        o=Object(materials=materials,path=path); i+=1
        while i<len(lines):
            t=shlex.split(lines[i]); i+=1
            if not t: continue
            if t[0]=="name": o.name=t[1]
            elif t[0]=="texture": o.texture=t[1]
            elif t[0]=="loc": o.loc=tuple(map(float,t[1:4]))
            elif t[0]=="rot":
                r=list(map(float,t[1:])); assert len(r)==9,(path,o.name,r)
                o.rot=tuple(tuple(r[j:j+3]) for j in (0,3,6))
            elif t[0]=="data":
                remaining=int(t[1])
                while remaining>0:
                    remaining-=len(lines[i].encode("latin-1"))+1; i+=1
            elif t[0]=="numvert":
                n=int(t[1]); o.verts=[tuple(map(float,line.split()[:3])) for line in lines[i:i+n]]; i+=n
            elif t[0]=="numsurf":
                for _ in range(int(t[1])):
                    flags=int(lines[i].split()[1],0); i+=1; mat=0
                    if lines[i].startswith("mat "): mat=int(lines[i].split()[1]); i+=1
                    assert lines[i].startswith("refs"),(path,i,lines[i])
                    count=int(lines[i].split()[1]); i+=1; refs=[]
                    for line in lines[i:i+count]:
                        r=line.split(); refs.append((int(r[0]),float(r[1]),float(r[2])))
                    i+=count
                    if flags&0xf==0 and len(refs)>=3: o.faces.append((mat,refs))
            elif t[0]=="kids":
                o.children=[obj() for _ in range(int(t[1]))]; return o
        return o
    return obj()

def flatten(o,rotation=IDENTITY,translation=(0.,0.,0.)):
    t=add(translation,mv(rotation,o.loc)); r=mm(rotation,o.rot)
    if o.verts: yield o,[add(mv(r,v),t) for v in o.verts]
    for child in o.children: yield from flatten(child,r,t)
def bounds(vs): return [tuple(min(p[k] for p in vs) for k in range(3)),tuple(max(p[k] for p in vs) for k in range(3))]

def triangulate(points):
    """Ear clip concave AC3D polygons; do not fan across intake/panel recesses."""
    n=(0.,0.,0.)
    for a,b in zip(points,points[1:]+points[:1]): n=add(n,cross(a,b))
    axis=max(range(3),key=lambda k:abs(n[k]))
    uv=[tuple(p[k] for k in range(3) if k!=axis) for p in points]
    area=sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(uv,uv[1:]+uv[:1])); sign=1 if area>=0 else -1
    def turn(a,b,c): return sign*((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]))
    ids=list(range(len(points))); out=[]
    while len(ids)>3:
        for j in range(len(ids)):
            a,b,c=ids[j-1],ids[j],ids[(j+1)%len(ids)]
            if turn(uv[a],uv[b],uv[c])<=1e-12: continue
            if any(all(turn(uv[x],uv[y],uv[p])>=-1e-10 for x,y in [(a,b),(b,c),(c,a)])
                   for p in ids if p not in (a,b,c) and all(abs(uv[p][0]-uv[q][0])+abs(uv[p][1]-uv[q][1])>1e-10 for q in (a,b,c))): continue
            out.append((a,b,c)); del ids[j]; break
        else:
            j=min(range(len(ids)),key=lambda k:abs(turn(uv[ids[k-1]],uv[ids[k]],uv[ids[(k+1)%len(ids)]])))
            a,b,c=ids[j-1],ids[j],ids[(j+1)%len(ids)]
            if abs(turn(uv[a],uv[b],uv[c]))>1e-7: raise ValueError(f"Non-simple polygon: {points}")
            del ids[j]
    if len(ids)==3: out.append(tuple(ids))
    return out


def ac_fg(p): return (p[0],-p[2],p[1])
def fg_ac(p): return (p[0],p[2],-p[1])
def game(p,kind):
    x,y,z=p
    return (-z*1.035,y*1.035,x*(17.1/15.39131)+0.15) if kind=="f18" else (-z*(14.7/15.25),y*(14.7/15.25),x*(21.9/22.07678)-10.95587)
def xml_pose(e):
    def f(k): return float(e.findtext(k,"0")) if e is not None else 0.
    a,b,c=[math.radians(f(k+"-deg")) for k in ("roll","pitch","heading")]
    rx=((1,0,0),(0,math.cos(a),-math.sin(a)),(0,math.sin(a),math.cos(a)))
    ry=((math.cos(b),0,math.sin(b)),(0,1,0),(-math.sin(b),0,math.cos(b)))
    rz=((math.cos(c),-math.sin(c),0),(math.sin(c),math.cos(c),0),(0,0,1))
    return mm(rz,mm(ry,rx)),(f("x-m"),f("y-m"),f("z-m"))
USED=set()
def models(path,root,rotation=IDENTITY,translation=(0.,0.,0.),trail=""):
    path=Path(path); USED.add(path)
    if path.suffix==".ac":
        for o,vs in flatten(read_ac(path)):
            yield o,[fg_ac(add(mv(rotation,ac_fg(v)),translation)) for v in vs],trail
        return
    e=ET.parse(path).getroot()
    def resolve(name): return root/name[len("Aircraft/Su-27/"):] if name.startswith("Aircraft/Su-27/") else path.parent/name
    own=e.findtext("path")
    if own: yield from models(resolve(own),root,rotation,translation,trail)
    for child in e.findall("model"):
        name=child.findtext("path")
        if not name: continue
        # Live instruments and the existing animated HOTAS are mounted in Rust.
        if ("/Instruments/" in name and "/HUD/" not in name) or ("/CockpitParts/" in name and "/Switches/" not in name): continue
        r,t=xml_pose(child.find("offsets"))
        yield from models(resolve(name),root,mm(rotation,r),add(translation,mv(rotation,t)),trail+"/"+name)

def qoi(image):
    """Lossless QOI: source resolution and UVs are preserved."""
    image=image.convert("RGB"); w,h=image.size
    out=bytearray(b"qoif"+struct.pack(">II",w,h)+bytes((3,0)))
    cache=[(0,0,0,0)]*64; prev=(0,0,0,255); run=0; data=image.tobytes()
    for j in range(0,len(data),3):
        p=(*data[j:j+3],255)
        if p==prev:
            run+=1
            if run==62 or j+3==len(data): out.append(0xc0|run-1); run=0
            continue
        if run: out.append(0xc0|run-1); run=0
        i=(p[0]*3+p[1]*5+p[2]*7+p[3]*11)%64
        if cache[i]==p: out.append(i)
        else:
            cache[i]=p; dr,dg,db=(p[k]-prev[k] for k in range(3))
            if -2<=dr<=1 and -2<=dg<=1 and -2<=db<=1: out.append(0x40|((dr+2)<<4)|((dg+2)<<2)|(db+2))
            elif -32<=dg<=31 and -8<=dr-dg<=7 and -8<=db-dg<=7: out.extend((0x80|(dg+32),((dr-dg+8)<<4)|(db-dg+8)))
            else: out.extend((0xfe,*p[:3]))
        prev=p
    out.extend(bytes((0,0,0,0,0,0,0,1)))
    return out

def clip_half(poly,axis,value,positive):
    out=[]
    for a,b in zip(poly,poly[1:]+poly[:1]):
        fa=(a[axis]-value)*(1 if positive else -1)
        fb=(b[axis]-value)*(1 if positive else -1)
        if fa>=-1e-10:out.append(a)
        if (fa>0) != (fb>0):
            t=fa/(fa-fb);out.append(tuple(x+(y-x)*t for x,y in zip(a,b)))
    return out
def without_rectangle(poly,x0,x1,z0,z1):
    """Cut an actual opening, preserving position, normal, colour and UV."""
    result=[]
    for axis,value,positive in [(0,x0,True),(0,x1,False),(2,z0,True),(2,z1,False)]:
        outside=clip_half(poly,axis,value,not positive)
        if len(outside)>=3:result.append(outside)
        poly=clip_half(poly,axis,value,positive)
        if len(poly)<3:break
    return result

def styled_texture(image, cockpit=False):
    """Clean flat colours, preserved aspect and sharp source markings."""
    from PIL import Image, ImageFilter
    image=image.convert("RGB")
    # Uniform scaling keeps UV shapes. Console legends keep their native grid.
    limit=2048
    scale=min(1.0,limit/max(image.size))
    if scale<1.0:
        image=image.resize(tuple(max(1,round(v*scale)) for v in image.size),Image.Resampling.LANCZOS)
    # Suppress painted speckle without softening the outlines of small legends.
    flat=image.filter(ImageFilter.MedianFilter(3))
    count=64 if cockpit else 48
    base=flat.quantize(colors=count-8,method=Image.Quantize.MEDIANCUT,dither=Image.Dither.NONE)
    # Reserve colours for small insignia, numbers, warning lights and handles:
    # an adaptive palette dominated by blue paint otherwise discards their red.
    colours=flat.resize((min(512,flat.width),min(512,flat.height)),Image.Resampling.NEAREST).getcolors(512*512)
    groups=[
        lambda r,g,b:r>120 and r>g*1.5 and r>b*1.4,
        lambda r,g,b:50<r<=120 and r>g*1.5 and r>b*1.4,
        lambda r,g,b:min(r,g,b)>215,
        lambda r,g,b:r>130 and g>90 and b<min(r,g)*0.65,
        lambda r,g,b:max(r,g,b)<50,
        lambda r,g,b:g>90 and g>r*1.25 and g>b*1.15,
        lambda r,g,b:r>160 and 45<g<140 and b<65,
        lambda r,g,b:150<min(r,g,b)<215 and max(r,g,b)-min(r,g,b)<25,
    ]
    defaults=[(210,30,28),(110,25,25),(240,240,230),(230,190,45),
        (22,26,28),(60,150,65),(220,100,25),(185,185,180)]
    accents=[max(((n,c) for n,c in colours if predicate(*c)),default=(0,fallback))[1]
        for predicate,fallback in zip(groups,defaults)]
    palette=base.getpalette()[:(count-8)*3]+[v for c in accents for v in c]
    fixed=Image.new("P",(1,1));fixed.putpalette(palette+palette[:3]*((768-len(palette))//3))
    return flat.quantize(palette=fixed,dither=Image.Dither.NONE).convert("RGB")

def su27_metal_colour(name, refs, normal):
    """Flat heat shields and nozzle petals; leave painted atlas islands intact.

    The rectangles are the bare engine shrouds in the pinned 2048 atlas,
    in top-left image coordinates. No source pixels or camouflage are changed.
    """
    if name=="EngineBlocks":return (.065,.075,.080)
    if name=="HULL_Hull_Upper.002":return (.29,.31,.32) if normal[2]<.65 else (.10,.115,.12)
    if name!="HULL_Hull_Upper":return None
    u=sum(r[1] for r in refs)/len(refs)
    v=1.-sum(r[2] for r in refs)/len(refs)
    if .663<=v<=.845 and (.456<=u<=.517 or .582<=u<=.644):
        return (.43,.45,.46)
    if .834<=u<=.995 and .003<=v<=.186:
        return (.43,.45,.46)
    return None

def generate(source,out):
    from PIL import Image
    import shutil
    out.mkdir(parents=True,exist_ok=True)
    report={}; textures={}
    def texture(o):
        if not o.texture:return None
        if o.texture=="panel-front-tex-2048-bevel.png":return None
        p=o.path.parent/o.texture
        if not p.exists():
            matches=list((source/"su27").rglob(Path(o.texture).name))
            if len(matches)==1:p=matches[0]
            else:return None
        if p not in textures:textures[p]=styled_texture(Image.open(p),cockpit=True); USED.add(p)
        return p
    for kind in ("f18","su27"):
        parts={n:[] for n in ("body","canopy","stab0","stab1","flap0","flap1","aileron0","aileron1","rudder0","rudder1","brake","gear0","gear1","gear2","cockpit","frame","nose_tyre")}
        texparts={}; objects=[]
        # Compact flat-colour exterior atlas; normalized UVs retain source shapes.
        skin_sources=[source/"f18/f18top.rgb",source/"f18/f18tail.rgb"] if kind=="f18" else [source/"su27/Models/USSR177IAP.png",source/"su27/Models/fabric.png"]
        images=[styled_texture(Image.open(p)) for p in skin_sources]
        for p in skin_sources:USED.add(p)
        sw=1
        while sw<sum(im.width for im in images):sw*=2
        sh=1
        while sh<max(im.height for im in images):sh*=2
        skin=Image.new("RGB",(sw,sh));tiles={};tx=0
        for path,im in zip(skin_sources,images):
            skin.paste(im,(tx,0));tiles[path.name]=(tx,0,im.width,im.height);tx+=im.width
        (out/(kind+"-skin.qoi")).write_bytes(qoi(skin))
        def emit(o,vs,part,textured=False):
            vs=[game(v,kind) for v in vs]; tex=texture(o) if textured else None
            arr=texparts.setdefault(tex,[]) if tex else parts[part]
            ext=None
            if not textured and part not in ("canopy","frame","cockpit") and o.texture:
                key=o.texture if kind=="f18" else ("fabric.png" if o.texture=="fabric.png" else "USSR177IAP.png")
                ext=tiles.get(key)
            for mat,refs in o.faces:
                ps=[vs[r[0]] for r in refs]
                for ids in triangulate(ps):
                    tri=[ps[i] for i in ids]
                    # Canopy_default also contains the old narrow cockpit tub.
                    # The new continuous tub replaces those interior side faces.
                    if kind=="f18" and part=="frame" and min(p[1] for p in tri)<.70:continue
                    cp=cross(sub(tri[1],tri[0]),sub(tri[2],tri[0]))
                    if dot(cp,cp)<1e-16:continue
                    n=unit(cp); col=o.materials[mat].rgb
                    if textured and not tex:col=(.32,.53,.56)
                    if part=="canopy":col=(.16,.22,.24)
                    elif not textured:
                        name=o.name.lower(); col=(.58,.62,.64) if kind=="f18" else (.62,.71,.77)
                        if "reactor" in name or "upper.002" in name:col=(.33,.34,.33)
                        if "tyre" in name or "rubber" in o.materials[mat].name.lower():col=(.04,.045,.05)
                        elif part.startswith("gear"):col=(.66,.69,.66)
                        if part=="frame":col=(.08,.10,.11)
                        centre=tuple(sum(p[k] for p in tri)/3 for k in range(3))
                        if kind=="f18" and part=="body" and -.7<centre[2]<1.2 and abs(centre[0])>.60 and centre[1]<.3 and abs(n[2])>.5:col=(.045,.055,.06)
                        if kind=="su27" and part=="body" and -4.<centre[2]<-1. and abs(centre[0])>.65 and centre[1]<0 and abs(n[2])>.7:col=(.045,.055,.06)
                        if kind=="su27" and part=="body" and sum(p[1] for p in tri)/3<-.65:col=(.74,.80,.82)
                    if ext:col=o.materials[mat].rgb
                    metal=su27_metal_colour(o.name,[refs[k] for k in ids],n) if kind=="su27" and part=="body" else None
                    if metal is not None:col=metal
                    vertices=[]
                    for k in ids:
                        uv=refs[k][1:]
                        if ext and metal is None:
                            tx,ty,tw,th=ext
                            uv=((tx+.5+uv[0]*(tw-1))/sw,(ty+.5+(1.-uv[1])*(th-1))/sh)
                        vertices.append((*ps[k],*n,*col,uv[0],1.-uv[1]) if tex else ((*ps[k],*n,*col,*uv) if ext and metal is None else (*ps[k],*n,*col,-1.,-1.)))
                    if kind=="su27" and part=="cockpit" and "/LeftPanels/" in o.path.as_posix():
                        for poly in without_rectangle(vertices,-.493,-.377,-6.198,-5.788):
                            for j in range(1,len(poly)-1):
                                for v in (poly[0],poly[j],poly[j+1]):arr.extend(v)
                    elif kind=="su27" and part=="cockpit" and o.name=="Panel.Main.004":
                        # The source front wall closes behind the new pedal
                        # position. Open both foot tunnels; the continuous Rust
                        # tub supplies their front firewall and lower floor.
                        for poly in without_rectangle(vertices,-.275,.275,-7.5,-6.10):
                            for j in range(1,len(poly)-1):
                                for v in (poly[0],poly[j],poly[j+1]):arr.extend(v)
                    elif kind=="su27" and part=="cockpit" and o.name.startswith("Panel.Cap"):
                        # The old coaming roof crosses the separate HDD. Open
                        # its case footprint; Rust supplies the continuous hood.
                        for poly in without_rectangle(vertices,.10,.36,-6.628,-6.138):
                            for j in range(1,len(poly)-1):
                                for v in (poly[0],poly[j],poly[j+1]):arr.extend(v)
                    else:
                        for v in vertices:arr.extend(v)
            objects.append({"name":o.name,"part":part,"source":o.path.relative_to(source).as_posix(),"bounds":bounds([vs[i] for i in sorted({r[0] for _,refs in o.faces for r in refs})]) if o.faces else bounds(vs)})
        if kind=="f18":
            path=source/"f18/f18.ac"; USED.add(path)
            for o,vs in flatten(read_ac(path)):
                name=o.name
                if name.startswith(("Sidewinder","ExternalFlame","InternalFlame","RightDoor","LeftDoor","FrontDoor","Ramp")):continue
                if name in ("RightWheel","LeftWheel","FrontWheel"):part="gear0" if name=="FrontWheel" else ("gear1" if name=="LeftWheel" else "gear2")
                elif name.startswith("Elevator"):part="stab0" if "Left" in name else "stab1"
                elif name.startswith("Flaps"):part="flap0" if "Left" in name else "flap1"
                elif name.startswith("Aileron"):part="aileron0" if "Left" in name else "aileron1"
                elif name.startswith("Rudder"):part="rudder0" if "Left" in name else "rudder1"
                elif name=="Canopy_Glass":part="canopy"
                elif name.startswith(("cube","cylinder","Stick","Truster","box")):continue
                else:part="body"
                emit(o,vs,part)
                if name=="Canopy_default":emit(o,vs,"frame")
        else:
            path=source/"su27/Models/Su-27.ac"; USED.add(path)
            for o,vs in flatten(read_ac(path)):
                name=o.name
                if name.startswith("canopy"):part="canopy"
                elif "Elevon" in name:part="stab0" if "Left" in name else "stab1"
                elif "Flaperon" in name:part="flap0" if "Left" in name else "flap1"
                elif "Rudder" in name:part="rudder0" if "Left" in name else "rudder1"
                elif "Speedbrake" in name:part="brake"
                elif "Gear-Door" in name or "Nose_Door" in name or name=="HULL_Hull_Upper.001":continue
                else:part="body"
                emit(o,vs,part)
                if name in ("Canopy_Front","Canopy_Rear"):emit(o,vs,"frame")
            path=source/"su27/Models/Gear.ac"; USED.add(path)
            for o,vs in flatten(read_ac(path)):
                if o.name in ("UC_Nose_Tyres","UC_Nose_Hubs"):
                    # Common upstream gear contains the naval twin nose wheels.
                    # Baseline Su-27 uses one 680 x 260 mm wheel.
                    if o.name=="UC_Nose_Tyres":
                        o.faces=[(mat,refs) for mat,refs in o.faces if all(vs[r[0]][2]>0 for r in refs)]
                    else:
                        o.faces=[(mat,refs) for mat,refs in o.faces if all(abs(vs[r[0]][2])<.14 for r in refs)]
                    sc=14.7/15.25;radial=(.34/sc)/.32174
                    centre=-2.29801+.34/sc
                    vs=[(7.80208+(x-7.80208)*radial,centre+(y+1.97627)*radial,
                         (z-.160475)*((.26/sc)/.17441) if o.name=="UC_Nose_Tyres" else z) for x,y,z in vs]
                elif o.name=="UC_Nose_WGuard":
                    vs=[(x,y+.031,z*.58) for x,y,z in vs]
                emit(o,vs,"gear0" if "Nose" in o.name else ("gear1" if "Left" in o.name else "gear2"))
                if o.name=="UC_Nose_Tyres":emit(o,vs,"nose_tyre")
            root=source/"su27"
            for o,vs,t in models(root/"Models/Su-27-Cockpit/Su-27SK-Cockpit.xml",root):
                if o.name in ("glass","HUD_Screen"):
                    if o.name=="HUD_Screen": print("HUD GLASS",[game(p,kind) for p in vs],flush=True)
                    continue
                if o.path.name=="Su-27SK-HUD.ac" and o.name in ("hud","hudframe.005","InBoardDspl_L.knob","knob","text"):continue
                # Replace the old front board, coaming and ground plane as one
                # assembly in Rust. Keeping both creates intersecting panels,
                # a diagram over the gauges and a floor through the pilot's feet.
                if o.name in ("Panel.Main.Front", "Panel.Main.Front.001",
                              "Panel.Main.Front.002", "Panel.Cap.left.001",
                              "Panel.wcs", "Plane-cockpit-GND"):continue
                emit(o,vs,"cockpit",True)
        data=bytearray(b"VRM1"+struct.pack("<I",len(parts)))
        for name,arr in parts.items():
            b=name.encode();data.extend(struct.pack("<I",len(b))+b+struct.pack("<I",len(arr))+struct.pack("<"+"f"*len(arr),*arr))
        data.extend(struct.pack("<I",len(texparts)))
        for tex,arr in texparts.items():
            img=qoi(textures[tex]);data.extend(struct.pack("<II",len(arr),len(img))+struct.pack("<"+"f"*len(arr),*arr)+img)
        (out/(kind+".vrm")).write_bytes(data)
        (out/(kind+".vrz")).write_bytes(b"VRZ1"+struct.pack("<I",len(data))+zlib.compress(data,9))
        report[kind]={"parts":{k:len(v)//33 for k,v in parts.items()},"textured_parts":len(texparts),"textures":[{"source":k.relative_to(source).as_posix(),"size":textures[k].size,"style":"flat-64"} for k in texparts],"objects":objects}
        print(kind,report[kind]["parts"],"textures",len(texparts),"bytes",len(data),flush=True)
    (out/"geometry.json").write_text(json.dumps(report,indent=2)+"\n", newline="\n")
    manifest=[]
    for p in sorted(USED,key=lambda p:p.relative_to(source).as_posix()):
        rel=p.relative_to(source); dest=out/"source"/rel; data=p.read_bytes()
        manifest.append({"path":rel.as_posix(),"sha256":hashlib.sha256(data).hexdigest()})
        dest.parent.mkdir(parents=True,exist_ok=True)
        if p.suffix in (".ac",".xml"):Path(str(dest)+".gz").write_bytes(gzip.compress(data,mtime=0))
        else:shutil.copyfile(p,dest)
    (out/"sources.json").write_text(json.dumps(manifest,indent=2)+"\n", newline="\n")
def main():
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument("--inventory",type=Path)
    ap.add_argument("--source-root",type=Path,default=ROOT/"target/model-sources"); ap.add_argument("--output",type=Path,default=ROOT/"assets/models")
    ap.add_argument("--unpack-source",type=Path,help="Restore bundled editable sources into this directory")
    args=ap.parse_args()
    if args.unpack_source:
        base=ROOT/"assets/models/source"
        for p in sorted(base.rglob("*")):
            if not p.is_file():continue
            rel=p.relative_to(base);dest=args.unpack_source/rel
            if p.suffix==".gz":dest=dest.with_suffix("")
            dest.parent.mkdir(parents=True,exist_ok=True)
            dest.write_bytes(gzip.decompress(p.read_bytes()) if p.suffix==".gz" else p.read_bytes())
        print("Editable model sources restored:",args.unpack_source)
    elif args.inventory:
        for o,vs in flatten(read_ac(args.inventory)): print(o.name,len(vs),len(o.faces),o.texture,bounds(vs))
    else:generate(args.source_root,args.output)
if __name__=="__main__":main()
