"""Original Baylee sanctuary lantern. Run in Blender 5.2; no external assets."""
import bpy
import math
from mathutils import Vector
from pathlib import Path

ROOT = Path("/Users/viktor/Projects/baylee")
scene = bpy.data.scenes.new("Baylee • sanctuary lantern")
bpy.context.window.scene = scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = 32
scene.cycles.use_denoising = True
scene.render.resolution_x = 600
scene.render.resolution_y = 800
scene.render.resolution_percentage = 100
scene.render.film_transparent = True
scene.world = bpy.data.worlds.new("Sanctuary night")
scene.world.use_nodes = True
scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0.12, 0.2, 0.32, 1)
scene.world.node_tree.nodes["Background"].inputs[1].default_value = 0.3

def material(name, color, metal=0, rough=0.35, emission=0):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*color,1)
    m.use_nodes = True
    p = m.node_tree.nodes.get("Principled BSDF")
    p.inputs["Base Color"].default_value = (*color,1)
    p.inputs["Metallic"].default_value = metal
    p.inputs["Roughness"].default_value = rough
    if emission:
        p.inputs["Emission Color"].default_value = (*color,1)
        p.inputs["Emission Strength"].default_value = emission
    return m

brass = material("Old champagne brass", (.32,.19,.07), .82, .28)
gold = material("Polished edges", (.72,.45,.15), .78, .22)
dark = material("Patinated teal enamel", (.014,.075,.07), .65)
glass = material("Honey light", (.95,.36,.035), .1, .3, 2.2)

def finish(obj,name,mat):
    obj.name = name
    obj.data.materials.append(mat)
    for poly in getattr(obj.data,"polygons",[]): poly.use_smooth = True
    return obj

def cylinder(name,r,depth,z,mat,vertices=64):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices,radius=r,depth=depth,location=(0,0,z))
    obj=finish(bpy.context.object,name,mat)
    bevel=obj.modifiers.new("Soft machined edges",'BEVEL')
    bevel.width=.012
    bevel.segments=3
    obj.modifiers.new("Weighted normals",'WEIGHTED_NORMAL')
    return obj

def curve(name,points,r,mat,cyclic=False):
    data=bpy.data.curves.new(name,'CURVE')
    data.dimensions='3D'
    data.bevel_depth=r
    data.bevel_resolution=3
    spline=data.splines.new('POLY')
    spline.points.add(len(points)-1)
    for p,co in zip(spline.points,points): p.co=(*co,1)
    spline.use_cyclic_u=cyclic
    obj=bpy.data.objects.new(name,data)
    scene.collection.objects.link(obj)
    obj.data.materials.append(mat)
    return obj

for radius,depth,z,mat in [(.39,.08,.04,dark),(.41,.025,.09,gold),(.32,.09,.15,brass),(.35,.026,.205,gold),(.29,.035,.25,gold),(.29,.05,1.03,brass),(.33,.028,1.07,gold)]:
    cylinder("Lantern • turned plinth",radius,depth,z,mat)
# Six luminous inset panes with visible open brass tracery.
cylinder("Lantern • luminous honey core",.225,.74,.64,glass,6)
for i in range(6):
    a=i*math.tau/6+math.pi/6
    x,y=.27*math.cos(a),.27*math.sin(a)
    curve("Lantern • upright",[(x,y,.25),(x,y,1.04)],.022,brass)
    # Two opposed, elongated botanical curls per pane.
    mid=a+math.pi/6
    for side in [-1,1]:
        points=[]
        for j in range(49):
            t=j/48
            along=side*.103*math.sin(math.pi*t)
            radial=.253
            points.append((radial*math.cos(mid)-along*math.sin(mid),radial*math.sin(mid)+along*math.cos(mid),.29+t*.68))
        curve("Lantern • petal tracery",points,.009,gold)
    points=[]
    for j in range(5):
        t=j*math.pi/2
        lateral=.065*math.sin(t)
        z=.64+.115*math.cos(t)
        points.append((.256*math.cos(mid)-lateral*math.sin(mid),.256*math.sin(mid)+lateral*math.cos(mid),z))
    curve("Lantern • diamond",points,.012,brass)
# Roof, radial ribs, bead finial.
bpy.ops.mesh.primitive_cone_add(vertices=64,radius1=.34,radius2=.09,depth=.27,location=(0,0,1.215))
finish(bpy.context.object,"Lantern • swept roof",dark)
for i in range(12):
    a=i*math.tau/12
    curve("Lantern • roof rib",[(.34*math.cos(a),.34*math.sin(a),1.08),(.21*math.cos(a),.21*math.sin(a),1.24),(.09*math.cos(a),.09*math.sin(a),1.35)],.009,gold)
cylinder("Lantern • crown",.10,.035,1.37,gold)
bpy.ops.mesh.primitive_uv_sphere_add(segments=24,ring_count=12,radius=.06,location=(0,0,1.43))
finish(bpy.context.object,"Lantern • finial",gold)
curve("Lantern • carry loop",[(.115*math.cos(i*math.tau/64),0,1.56+.15*math.sin(i*math.tau/64)) for i in range(64)],.018,brass,True)
# Tiny paired pointed ears, a quiet Baylee signature.
for s in [-1,1]:
    curve("Lantern • guardian ears",[(s*.22,0,1.22),(s*.18,0,1.45),(s*.10,0,1.32)],.014,gold)

def area(name,loc,power,color,size):
    data=bpy.data.lights.new(name,'AREA');data.energy=power;data.color=color;data.shape='DISK';data.size=size
    ob=bpy.data.objects.new(name,data);scene.collection.objects.link(ob);ob.location=loc
    ob.rotation_euler=(Vector((0,0,.8))-ob.location).to_track_quat('-Z','Y').to_euler()
area("Warm key",(-3,-4,4),480,(1,.72,.4),3)
area("Moon rim",(2,1,3),650,(.32,.65,1),2)
area("Front softbox",(1,-3,2),180,(1,.88,.65),2)
camera=bpy.data.cameras.new("Lantern portrait");ob=bpy.data.objects.new("Lantern portrait",camera);scene.collection.objects.link(ob)
ob.location=(2.8,-6,2.8);ob.rotation_euler=(Vector((0,0,.86))-ob.location).to_track_quat('-Z','Y').to_euler()
camera.type='ORTHO';camera.ortho_scale=2.08;scene.camera=ob
scene.view_settings.view_transform='AgX'
scene.render.image_settings.file_format='PNG'
scene.render.image_settings.color_mode='RGBA'
scene.render.filepath=str(ROOT/"crates/baylee-client/assets/scenes/wayfinder-lantern.png")
# Apply scale and convert curves for a portable editable GLB.
for ob in bpy.data.objects: ob.select_set(False)
for ob in scene.objects:
    if ob.type in {'MESH','CURVE'}: ob.select_set(True)
bpy.context.view_layer.objects.active=next(ob for ob in scene.objects if ob.type=='MESH')
bpy.ops.object.convert(target='MESH')
bpy.ops.object.transform_apply(location=False,rotation=False,scale=True)
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/"art/baylee/wayfinder-lantern.blend"))
bpy.ops.export_scene.gltf(filepath=str(ROOT/"art/baylee/wayfinder-lantern.glb"),use_selection=True,use_active_scene=True,export_format='GLB',export_yup=True)
bpy.ops.render.render(write_still=True)
result={'scene':scene.name,'objects':len(scene.objects),'render':scene.render.filepath,'blend':bpy.data.filepath}
