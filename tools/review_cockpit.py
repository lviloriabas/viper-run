"""Inspect the exact game cockpit meshes exported with --export-cockpit."""
import bpy, struct, sys, math
from pathlib import Path
from mathutils import Vector
root=Path(sys.argv[sys.argv.index("--")+1])
out=root/"blender";out.mkdir(exist_ok=True)
scene=bpy.context.scene
scene.render.engine="BLENDER_WORKBENCH"
scene.display.shading.light="STUDIO"
scene.display.shading.color_type="VERTEX"
scene.display.shading.show_shadows=True
scene.display.shading.show_cavity=True
scene.display.shading.cavity_type="BOTH"
scene.display.shading.background_type="WORLD"
scene.world.color=(.1,.12,.14)
scene.render.resolution_x=1600;scene.render.resolution_y=1200
scene.render.resolution_percentage=100
kinds=("f18","su27","mig29")
if "--jet" in sys.argv:
    jet=sys.argv[sys.argv.index("--jet")+1]
    if jet not in kinds:raise ValueError("Unsupported cockpit "+jet)
    kinds=(jet,)
for kind in kinds:
    bpy.ops.object.select_all(action="SELECT");bpy.ops.object.delete(use_global=False)
    data=(root/(kind+"-cockpit.vrm")).read_bytes();at=4
    def u32():
        global at
        n=struct.unpack_from("<I",data,at)[0];at+=4;return n
    count=u32()
    for _ in range(count):
        n=u32();name=data[at:at+n].decode();at+=n
        n=u32();a=struct.unpack_from("<"+"f"*n,data,at);at+=n*4
        vs=[(a[k],-a[k+2],a[k+1]) for k in range(0,n,11)]
        fs=[tuple(range(k,k+3)) for k in range(0,len(vs),3)]
        me=bpy.data.meshes.new(name);me.from_pydata(vs,[],fs)
        ob=bpy.data.objects.new(name,me);scene.collection.objects.link(ob)
        colors=me.color_attributes.new(name="source colors",type="FLOAT_COLOR",domain="CORNER")
        for i,col in enumerate(colors.data):col.color=(*a[i*11+6:i*11+9],1)
    # Keep a second collection with the starboard shell cut away below
    # the sill, so the leg wells can be inspected in a true side view.
    cutaways=[]
    for ob in list(scene.objects):
        if ob.type!="MESH":continue
        mesh=ob.data.copy();clone=bpy.data.objects.new(ob.name+" cutaway",mesh)
        scene.collection.objects.link(clone)
        import bmesh
        bm=bmesh.new();bm.from_mesh(mesh)
        bmesh.ops.delete(bm,geom=[f for f in bm.faces if all(v.co.x>.245 for v in f.verts)],context="FACES")
        bm.to_mesh(mesh);bm.free()
        clone.hide_render=True;cutaways.append(clone)
    originals=[ob for ob in scene.objects if ob.type=="MESH" and ob not in cutaways]
    camdata=bpy.data.cameras.new("Pilot inspection");cam=bpy.data.objects.new("Pilot inspection",camdata)
    scene.collection.objects.link(cam);scene.camera=cam
    for name,pos,aim,ortho in [
        ("pilot",(0,3.12 if kind=="su27" else 3.24,1.25),(0,4.05,.77),False),
        ("top",(0,3.65,4.0),(0,3.65,.5),2.5),
        ("side",(3.8,3.8,.8),(0,3.8,.7),2.5),
        ("front",(0,6.2,.95),(0,3.6,.7),2.5),
        ("leg-clearance",(3.8,3.9,.65),(0,3.9,.65),2.5),
        ("bow",(0,2.8,1.30),(0,3.8,1.30),1.25),
        ("hud-lead",(-.35,3.30,1.42),(-.07,3.82,1.16),.66),
        ("stick-clearance",(2.2,3.78,.62),(0,3.78,.62),1.1)]:
        for ob in originals:ob.hide_render=name in ("leg-clearance","stick-clearance")
        for ob in cutaways:ob.hide_render=name not in ("leg-clearance","stick-clearance")
        camdata.type="ORTHO" if ortho else "PERSP"
        camdata.ortho_scale=ortho if ortho else 2.5;camdata.lens=20
        cam.location=pos;cam.rotation_euler=(Vector(aim)-cam.location).to_track_quat("-Z","Y").to_euler()
        scene.render.filepath=str(out/(kind+"-"+name+".png"))
        bpy.ops.render.render(write_still=True)
    for ob in originals:
        ob.hide_render=False;ob.hide_set(False)
    for ob in cutaways:
        ob.hide_render=True;ob.hide_set(True)
    bpy.ops.wm.save_as_mainfile(filepath=str(out/(kind+"-cockpit.blend")))
