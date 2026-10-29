"""Render the OpenHornet CAD reference in metres with Blender."""
import bpy,sys
from pathlib import Path
from mathutils import Vector
root=Path(sys.argv[sys.argv.index("--")+1]);out=root/"cad-blender";out.mkdir(exist_ok=True)
bpy.ops.object.select_all(action="SELECT");bpy.ops.object.delete(use_global=False)
bpy.ops.wm.obj_import(filepath=str(root/"openhornet-reference.obj"),forward_axis="Y",up_axis="Z")
for ob in bpy.context.selected_objects:ob.scale=(.001,.001,.001)
scene=bpy.context.scene;scene.render.engine="BLENDER_WORKBENCH"
scene.display.shading.light="STUDIO";scene.display.shading.color_type="SINGLE"
scene.display.shading.single_color=(.4,.48,.49);scene.display.shading.show_cavity=True
scene.display.shading.cavity_type="BOTH";scene.display.shading.show_shadows=True
scene.render.resolution_x=1600;scene.render.resolution_y=1200;scene.render.resolution_percentage=100
data=bpy.data.cameras.new("Reference");cam=bpy.data.objects.new("Reference",data)
scene.collection.objects.link(cam);scene.camera=cam;data.type="ORTHO";data.ortho_scale=2.4
for name,pos,aim in [
    ("forward",(0,5.0,3.8),(0,7.0,3.2)),("rear",(0,8.0,3.8),(0,6.0,3.2)),
    ("top",(0,6.2,6.0),(0,6.2,3.2)),("side",(4,6.2,3.3),(0,6.2,3.3))]:
    cam.location=pos;cam.rotation_euler=(Vector(aim)-cam.location).to_track_quat("-Z","Y").to_euler()
    scene.render.filepath=str(out/(name+".png"));bpy.ops.render.render(write_still=True)
bpy.ops.wm.save_as_mainfile(filepath=str(out/"openhornet-reference.blend"))
