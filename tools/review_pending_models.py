"""Review exported assembled meshes beside the pinned FlightGear references.

blender -b -t 4 --python tools/review_pending_models.py -- target/todo-review/models
"""
import bpy, math, struct, sys
from pathlib import Path
from mathutils import Vector

root=Path(sys.argv[sys.argv.index('--')+1]).resolve()
repo=Path(__file__).resolve().parents[1]
out=root/'blender'
out.mkdir(exist_ok=True)
scene=bpy.context.scene
scene.render.engine='BLENDER_WORKBENCH'
scene.display.shading.color_type='VERTEX'
scene.display.shading.light='STUDIO'
scene.display.shading.show_shadows=True
scene.display.shading.show_cavity=True
scene.render.resolution_x=1200
scene.render.resolution_y=850
scene.render.resolution_percentage=100

def load(path,collection,prefix=''):
    data=path.read_bytes()
    at=4
    def u32():
        nonlocal at
        n=struct.unpack_from('<I',data,at)[0];at+=4
        return n
    for _ in range(u32()):
        n=u32();name=data[at:at+n].decode();at+=n
        n=u32();a=struct.unpack_from('<'+'f'*n,data,at);at+=n*4
        if not n:continue
        vs=[(a[k],-a[k+2],a[k+1]) for k in range(0,n,11)]
        faces=[tuple(range(k,k+3)) for k in range(0,len(vs),3)]
        mesh=bpy.data.meshes.new(prefix+name)
        mesh.from_pydata(vs,[],faces)
        ob=bpy.data.objects.new(prefix+name,mesh)
        collection.objects.link(ob)
        colors=mesh.color_attributes.new(name='paint',type='FLOAT_COLOR',domain='CORNER')
        for i,c in enumerate(colors.data):c.color=(*a[i*11+6:i*11+9],1)
        if name.startswith('gear'):ob.hide_render=True

for jet in ('f18','su27','f18e','ea18g'):
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    for c in list(bpy.data.collections):
        if c.name!='Collection':bpy.data.collections.remove(c)
    game=bpy.data.collections.new('Assembled game geometry')
    source=bpy.data.collections.new('Pinned source reference')
    cockpit=bpy.data.collections.new('Pilot and cockpit')
    for c in (game,source,cockpit):scene.collection.children.link(c)
    load(root/(jet+'-airframe.vrm'),game)
    load(repo/'assets/models'/('su27.vrm' if jet=='su27' else 'f18.vrm'),source,'reference-')
    source.hide_render=True
    source.hide_viewport=True
    load(root/(jet+'-cockpit.vrm'),cockpit,'cockpit-')
    cockpit.hide_render=True
    cockpit.hide_viewport=True
    camera=bpy.data.cameras.new('Orthographic inspection')
    ob=bpy.data.objects.new('Orthographic inspection',camera)
    scene.collection.objects.link(ob)
    scene.camera=ob
    camera.type='ORTHO';camera.ortho_scale=25 if jet=='su27' else 21
    for name,pos,aim in [('top',(0,0,38),(0,0,0)),('side',(35,0,3),(0,0,.5)),
                         ('front',(0,35,3),(0,0,.5)),('quarter',(25,28,18),(0,0,.5))]:
        span=25 if jet=='su27' else 21
        camera.ortho_scale=span*scene.render.resolution_x/scene.render.resolution_y if name=='top' else span
        ob.location=pos
        ob.rotation_euler=(Vector(aim)-ob.location).to_track_quat('-Z','Y').to_euler()
        scene.render.filepath=str(out/(jet+'-'+name+'.png'))
        bpy.ops.render.render(write_still=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out/(jet+'.blend')))
