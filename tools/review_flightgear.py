"""Blender background inspection of the exact embedded game geometry.
Run blender -b -t 4 --python tools/review_flightgear.py -- [output directory].
"""
import bpy, struct, sys, math
from pathlib import Path
from mathutils import Vector
root=Path(__file__).resolve().parents[1]
out=Path(sys.argv[sys.argv.index("--")+1]) if "--" in sys.argv else root/"target/model-review/blender"
out.mkdir(parents=True,exist_ok=True)
scene=bpy.context.scene
scene.render.engine="BLENDER_WORKBENCH"
scene.display.shading.light="STUDIO"
scene.display.shading.color_type="MATERIAL"
scene.display.shading.show_shadows=True
scene.display.shading.show_cavity=True
scene.display.shading.cavity_type="BOTH"
scene.display.shading.background_type="WORLD"
scene.world.color=(.12,.14,.16)
scene.render.resolution_x=1400;scene.render.resolution_y=1000;scene.render.resolution_percentage=100
for kind in ("f18","su27"):
    bpy.ops.object.select_all(action="SELECT");bpy.ops.object.delete(use_global=False)
    data=(root/"assets/models"/(kind+".vrm")).read_bytes(); i=4
    def u32():
        global i
        n=struct.unpack_from("<I",data,i)[0];i+=4;return n
    count=u32()
    for j in range(count):
        n=u32(); name=data[i:i+n].decode(); i+=n
        n=u32();a=struct.unpack_from("<"+"f"*n,data,i);i+=n*4
        if not n:continue
        vs=[(a[k],-a[k+2],a[k+1]) for k in range(0,n,11)]
        fs=[tuple(range(k,k+3)) for k in range(0,len(vs),3)]
        me=bpy.data.meshes.new(name);me.from_pydata(vs,[],fs)
        ob=bpy.data.objects.new(name,me);bpy.context.collection.objects.link(ob)
        mat=bpy.data.materials.new(kind+name);mat.diffuse_color=(*a[6:9],1)
        if name=="canopy":mat.diffuse_color=(.10,.23,.28,1)
        ob.data.materials.append(mat)
        if name in ("cockpit","frame","nose_tyre") or name.startswith("gear"):ob.hide_render=True
    camdata=bpy.data.cameras.new("Inspection");cam=bpy.data.objects.new("Inspection",camdata)
    scene.collection.objects.link(cam);scene.camera=cam;camdata.type="ORTHO";camdata.ortho_scale=26 if kind=="su27" else 20
    for name,pos in (("top",(0,0,40)),("side",(35,0,4)),("front",(0,35,3)),("three-quarter",(24,30,18))):
        cam.location=pos; cam.rotation_euler=(Vector((0,0,.3))-cam.location).to_track_quat("-Z","Y").to_euler()
        scene.render.filepath=str(out/(kind+"-"+name+".png"))
        bpy.ops.render.render(write_still=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out/(kind+".blend")))
