"""Blender views of the exact seat, handle and pilot exported by the game."""
import bpy, struct, sys
from pathlib import Path
from mathutils import Vector
root=Path(sys.argv[sys.argv.index("--")+1])
out=root/"blender";out.mkdir(exist_ok=True)
scene=bpy.context.scene
bpy.ops.object.select_all(action="SELECT");bpy.ops.object.delete(use_global=False)
scene.render.engine="BLENDER_WORKBENCH"
scene.display.shading.light="STUDIO";scene.display.shading.color_type="VERTEX"
scene.display.shading.show_cavity=True;scene.display.shading.cavity_type="BOTH"
scene.display.shading.background_type="WORLD";scene.world.color=(.11,.13,.16)
scene.render.resolution_x=1600;scene.render.resolution_y=1000
scene.render.resolution_percentage=100
data=(root/"ejection.vrm").read_bytes();at=4
def u32():
    global at
    n=struct.unpack_from("<I",data,at)[0];at+=4;return n
objects=[]
for _ in range(u32()):
    n=u32();name=data[at:at+n].decode();at+=n
    n=u32();a=struct.unpack_from("<"+"f"*n,data,at);at+=n*4
    vs=[(a[k],-a[k+2],a[k+1]) for k in range(0,n,11)]
    me=bpy.data.meshes.new(name);me.from_pydata(vs,[],[tuple(range(k,k+3)) for k in range(0,len(vs),3)])
    ob=bpy.data.objects.new(name,me);scene.collection.objects.link(ob);objects.append(ob)
    colors=me.color_attributes.new(name="game colors",type="FLOAT_COLOR",domain="CORNER")
    for i,c in enumerate(colors.data):c.color=(*a[i*11+6:i*11+9],1)
camdata=bpy.data.cameras.new("Inspection");cam=bpy.data.objects.new("Inspection",camdata)
scene.collection.objects.link(cam);scene.camera=cam;camdata.type="ORTHO"
for jet in ["f16","mig29","f18","su27"]:
    for ob in objects:
        ob.hide_render=not ob.name.startswith(jet+"-")
        ob.location.x={"rest":-2.1,"pulled":-.7,"separating":.7,"hanging":2.1}[ob.name.split("-")[1]]
    camdata.ortho_scale=6.4;cam.location=(3.0,6.0,2.3)
    cam.rotation_euler=(Vector((0,.15,-.1))-cam.location).to_track_quat("-Z","Y").to_euler()
    scene.render.filepath=str(out/(jet+"-ejection-poses.png"));bpy.ops.render.render(write_still=True)
    for ob in objects:
        ob.hide_render=not ob.name.startswith(jet+"-rest-") or ob.name.endswith("-pilot");ob.location.x=0
    camdata.ortho_scale=.68;cam.location=(.30,1.2,.35)
    cam.rotation_euler=(Vector((0,.46,-.04))-cam.location).to_track_quat("-Z","Y").to_euler()
    scene.render.filepath=str(out/(jet+"-ejection-handle.png"));bpy.ops.render.render(write_still=True)
for ob in objects:ob.hide_render=True
bpy.ops.wm.save_as_mainfile(filepath=str(out/"ejection.blend"))
