"""Blender scenes with final cockpit, movable skins, stores and source reference."""
import bpy,math,struct,sys
from pathlib import Path
from mathutils import Vector

repo=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(repo/'tools'))
import import_flightgear as fg
import import_superhornet as sh
root=Path(sys.argv[sys.argv.index('--')+1]).resolve();out=root/'blender';out.mkdir(exist_ok=True)
scene=bpy.context.scene;scene.render.engine='BLENDER_WORKBENCH'
scene.display.shading.color_type='VERTEX';scene.display.shading.light='STUDIO'
scene.display.shading.show_cavity=True;scene.display.shading.show_shadows=True
scene.render.resolution_x=1440;scene.render.resolution_y=1000;scene.render.resolution_percentage=100

def read(path,collection,offset=(0,0,0),prefix=''):
    data=path.read_bytes();at=4;made=[]
    def u32():
        nonlocal at
        n=struct.unpack_from('<I',data,at)[0];at+=4;return n
    for _ in range(u32()):
        n=u32();name=data[at:at+n].decode();at+=n;n=u32()
        values=struct.unpack_from('<'+'f'*n,data,at);at+=n*4
        if not n:continue
        vs=[(values[i]+offset[0],-values[i+2]-offset[2],values[i+1]+offset[1]) for i in range(0,n,11)]
        m=bpy.data.meshes.new(prefix+name);m.from_pydata(vs,[],[tuple(range(i,i+3)) for i in range(0,len(vs),3)])
        ob=bpy.data.objects.new(prefix+name,m);collection.objects.link(ob);made.append(ob)
        colours=m.color_attributes.new(name='flat paint',type='FLOAT_COLOR',domain='CORNER')
        for i,c in enumerate(colours.data):c.color=(*values[i*11+6:i*11+9],1)
    return made

for kind in ('f18','f18e','ea18g'):
    bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
    for c in list(bpy.data.collections):
        if c.name!='Collection':bpy.data.collections.remove(c)
    groups={}
    for name in ('Fixed and movable exterior','Cockpit and pilot','Pinned reference'):
        c=bpy.data.collections.new(name);scene.collection.children.link(c);groups[name]=c
    source=groups['Pinned reference'];source.hide_render=True
    exterior=read(root/(kind+'-airframe.vrm'),groups['Fixed and movable exterior'])
    sy=4.88/(5.13838+.01097);sz=18.3/(9.18164+9.08501)
    eye=(0,1.00395,-3.6275048) if kind=='f18' else (0,(2.85-2.2)*sy,-3.70*sz+.15)
    cockpit=read(root/(kind+'-cockpit.vrm'),groups['Cockpit and pilot'],(eye[0],eye[1]-1.25,eye[2]+3.24))
    # The pinned source envelope is independent of the game's extra closures,
    # reduced tyres, single-seat spine and mounted inventory meshes.
    if kind=='f18':read(repo/'assets/models/f18.vrm',source,prefix='reference-')
    else:
        path=repo/'assets/models/source/superhornet/Models/F-18F.ac.gz'
        for obj,vertices in fg.flatten(fg.read_ac(path)):
            vs=[sh.point(v) for v in vertices]
            vs=[(p[0],-p[2],p[1]) for p in vs];faces=[]
            for _,refs in obj.faces:
                for tri in fg.triangulate([vertices[r[0]] for r in refs]):faces.append(tuple(refs[i][0] for i in tri))
            mesh=bpy.data.meshes.new('Original '+obj.name);mesh.from_pydata(vs,[],faces)
            ob=bpy.data.objects.new('Original '+obj.name,mesh);source.objects.link(ob)
    source.hide_viewport=True
    for name in ('navy-canopy-reference.jpg','war-thunder-e.jpg'):
        file=repo/'target/canopy-super-review'/name
        if not file.exists():file=repo/'target/model-review/texture-bow'/name
        if file.exists():
            image=bpy.data.images.load(str(file),check_existing=True)
            ob=bpy.data.objects.new('Visual reference '+name,None);source.objects.link(ob)
            ob.empty_display_type='IMAGE';ob.data=image;ob.empty_display_size=7;ob.location=(-17,0,4)
    data=bpy.data.cameras.new('Inspection camera');cam=bpy.data.objects.new('Inspection camera',data)
    scene.collection.objects.link(cam);scene.camera=cam
    # Plan views include the full length at this landscape aspect.
    ex,ey,ez=eye;pilot=(ex,-ez,ey)
    views=[('top',(0,0,36),(0,0,.3),28),('front',(0,34,3),(0,0,.3),20),
           ('side',(32,0,4),(0,0,.3),23),('quarter',(24,28,18),(0,0,.3),23),
           ('mounts-bottom',(12,14,-11),(0,-1.2,-.25),17),
           ('pilot',pilot,(pilot[0],pilot[1]+1,pilot[2]-.05),0),
           ('bow',(pilot[0],pilot[1]-.55,pilot[2]+.10),(pilot[0],pilot[1]+.7,pilot[2]+.10),1.45)]
    for name,pos,aim,scale in views:
        for ob in exterior:ob.hide_render=name in ('pilot','bow') or ob.name.startswith('gear-')
        for ob in cockpit:ob.hide_render=name not in ('pilot','bow')
        data.type='ORTHO' if scale else 'PERSP';data.ortho_scale=scale or 2;data.lens=22
        cam.location=pos;cam.rotation_euler=(Vector(aim)-cam.location).to_track_quat('-Z','Y').to_euler()
        scene.render.filepath=str(out/(kind+'-'+name+'.png'));bpy.ops.render.render(write_still=True)
    for ob in exterior:ob.hide_render=ob.name.startswith('gear-')
    for ob in cockpit:ob.hide_render=True
    bpy.ops.wm.save_as_mainfile(filepath=str(out/(kind+'-assembled-with-references.blend')))
