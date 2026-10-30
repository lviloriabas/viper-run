"""Textured Blender review of exact game meshes and pinned source envelopes."""
from pathlib import Path
import json,math,struct,sys
import bpy
from mathutils import Vector

repo=Path(__file__).resolve().parents[1];sys.path.insert(0,str(repo/'tools'))
import import_flightgear as fg
import import_superhornet as sh
root=Path(sys.argv[sys.argv.index('--')+1]).resolve();out=root/'blender';out.mkdir(exist_ok=True)
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.samples=8
scene.render.resolution_x=1440;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
scene.view_settings.view_transform='Raw';scene.world.color=(.15,.17,.20)
sources=json.loads((repo/'assets/models/livery-sources.json').read_text())

def material(name,image=None):
    m=bpy.data.materials.new(name);m.use_nodes=True;n=m.node_tree.nodes;l=m.node_tree.links
    # Match the game's per-vertex lighting. Cycles otherwise flips back-face
    # shading on mirrored patches that intentionally store outward normals.
    shader=n.new('ShaderNodeEmission');l.new(shader.outputs[0],n.get('Material Output').inputs['Surface'])
    colour=n.new('ShaderNodeVertexColor');colour.layer_name='paint multiplier'
    if image:
        tex=n.new('ShaderNodeTexImage');tex.image=image;tex.interpolation='Closest'
        mul=n.new('ShaderNodeMixRGB');mul.blend_type='MULTIPLY';mul.inputs[0].default_value=1
        l.new(tex.outputs['Color'],mul.inputs[1]);l.new(colour.outputs['Color'],mul.inputs[2]);l.new(mul.outputs[0],shader.inputs['Color'])
    else:l.new(colour.outputs['Color'],shader.inputs['Color'])
    return m

def game(path,collection,flat,paint):
    data=path.read_bytes();at=4;made=[]
    def u32():
        nonlocal at
        n=struct.unpack_from('<I',data,at)[0];at+=4;return n
    for _ in range(u32()):
        n=u32();name=data[at:at+n].decode();at+=n;count=u32()
        values=struct.unpack_from('<'+'f'*count,data,at);at+=count*4
        if not count:continue
        vs=[(values[i],-values[i+2],values[i+1]) for i in range(0,count,11)]
        m=bpy.data.meshes.new(name);m.from_pydata(vs,[],[tuple(range(i,i+3)) for i in range(0,len(vs),3)])
        ob=bpy.data.objects.new(name,m);collection.objects.link(ob);made.append(ob)
        m.materials.append(flat);m.materials.append(paint)
        colours=m.color_attributes.new(name='paint multiplier',type='FLOAT_COLOR',domain='CORNER');uv=m.uv_layers.new()
        normals=[]
        for i,c in enumerate(colours.data):
            j=i*11;nx,ny,nz=values[j+3:j+6];sun=Vector((.42,.62,-.66)).normalized()
            light=.36+.16*ny+.80*max(0.,nx*sun.x+ny*sun.y+nz*sun.z)
            c.color=(*(v*light for v in values[j+6:j+9]),1);uv.data[i].uv=(values[j+9],1-values[j+10])
            normals.append((values[j+3],-values[j+5],values[j+4]))
        m.normals_split_custom_set(normals)
        for i,p in enumerate(m.polygons):p.material_index=1 if all(values[(i*3+k)*11+9]>=0 for k in range(3)) else 0
    return made

for jet in ['f16','mig29','f18e','ea18g']:
    bpy.ops.object.select_all(action='SELECT');bpy.ops.object.delete(use_global=False)
    for c in list(bpy.data.collections):
        if c.name!='Collection':bpy.data.collections.remove(c)
    geometry=bpy.data.collections.new('Painted game geometry');scene.collection.children.link(geometry)
    reference=bpy.data.collections.new('FlightGear source, comparison');scene.collection.children.link(reference)
    image=bpy.data.images.load(str(root/(jet+'-skin.png')),check_existing=True);image.colorspace_settings.name='Non-Color';image.pack()
    flat=material(jet+' special materials');paint=material(jet+' source paint',image)
    objects=game(root/(jet+'-airframe.vrm'),geometry,flat,paint)
    if jet in sources:
        folder=repo/'assets/models/source/liveries'/jet
        ac=folder/'Models'/('f16.ac.gz' if jet=='f16' else 'MiG-29.ac.gz')
        tr=sources[jet]['source_transform'];scale=tr['scale'];offset=tr['offset']
        def point(p):return (-p[2]*scale[0]+offset[0],p[1]*scale[1]+offset[1],p[0]*scale[2]+offset[2])
    else:
        ac=repo/'assets/models/source/superhornet/Models/F-18F.ac.gz';point=sh.point
    for ob,vs in fg.flatten(fg.read_ac(ac)):
        verts=[point(v) for v in vs];faces=[]
        for _,refs in ob.faces:
            for ids in fg.triangulate([verts[r[0]] for r in refs]):faces.append(tuple(refs[i][0] for i in ids))
        m=bpy.data.meshes.new('Source '+ob.name);m.from_pydata([(p[0],-p[2],p[1]) for p in verts],[],faces)
        o=bpy.data.objects.new('Source '+ob.name,m);reference.objects.link(o);o.display_type='WIRE'
    reference.hide_render=True;reference.hide_viewport=True
    bpy.ops.object.light_add(type='SUN',location=(10,10,30));bpy.context.object.rotation_euler=(.35,-.4,-.3);bpy.context.object.data.energy=2.3
    data=bpy.data.cameras.new('Paint review camera');cam=bpy.data.objects.new('Paint review camera',data);scene.collection.objects.link(cam);scene.camera=cam
    data.type='ORTHO';data.ortho_scale=24
    for name,pos,aim in [('top',(0,0,35),(0,0,0)),('left',(-30,0,5),(0,0,.7)),('right',(30,0,5),(0,0,.7)),('bottom',(0,-2,-35),(0,-2,0)),('quarter',(24,30,20),(0,0,.4))]:
        for ob in objects:ob.hide_render=ob.name.startswith('gear-')
        cam.location=pos;cam.rotation_euler=(Vector(aim)-cam.location).to_track_quat('-Z','Y').to_euler()
        scene.render.filepath=str(out/(jet+'-'+name+'.png'));bpy.ops.render.render(write_still=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out/(jet+'-paint-and-source.blend')))
