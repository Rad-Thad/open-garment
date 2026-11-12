import json
import math
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'scratch' / 'solver_site'))
sys.path.insert(0, str(ROOT))

import bpy
import numpy as np
from mathutils import Vector

import devil_designer
from devil_designer import garment, xpbd, xpbd_live
from devil_designer.core import Document
from devil_designer.presets import measured_to_preset


def check(condition, message):
    if not condition:
        raise AssertionError(message)
    print('PASS:', message, flush=True)


def points(simulation):
    return xpbd_live.evaluated_points(bpy.context, simulation)


def run_frames(scene, count):
    for frame in range(scene.frame_current + 1, scene.frame_current + count + 1):
        scene.frame_set(frame)


def seam_gap(simulation):
    coordinates = points(simulation)
    return max(math.dist(coordinates[first], coordinates[second]) for first, second in json.loads(simulation['dd_sewing_pairs']))


def stretch(simulation):
    coordinates = points(simulation)
    rest = simulation.data.attributes['rest_length'].data
    ratios = sorted(math.dist(coordinates[edge.vertices[0]], coordinates[edge.vertices[1]]) / rest[edge.index].value
                    for edge in simulation.data.edges if not edge.is_loose and rest[edge.index].value > 1e-6)
    return ratios[int(len(ratios) * .99)]


devil_designer.register()
scene = bpy.context.scene
scene.dd_solver = 'GPU'
scene.dd_spacing = .025
scene.render.fps = 30
scene.frame_set(scene.frame_start)
check('FINISHED' in bpy.ops.devil_designer.demo(), 'demo garment created')
for spacing, frames in ((.04, 30), (.02, 120), (.01, 120), (.005, 30)):
    scene.dd_spacing = spacing
    scene.pop('dd_last_error', None)
    simulation = garment.compile_garment(bpy.context)
    start_pose = points(simulation).copy()
    run_frames(scene, frames)
    check(simulation.get('dd_steps') == frames and not scene.get('dd_last_error'), f'spacing {spacing} steps the GPU solver on every frame')
    check(np.abs(points(simulation) - start_pose).max() > .001, f'spacing {spacing} moves the cloth')
    if frames == 120:
        check(stretch(simulation) < 1.15, f'spacing {spacing} cloth stretch stays in range')
        check(seam_gap(simulation) < .03, f'spacing {spacing} sewn seams close on the body')
    garment.reset(bpy.context)
scene.dd_spacing = .025
simulation = garment.compile_garment(bpy.context)
check(simulation.get('dd_solver') == 'GPU', 'simulation object is owned by the GPU backend')
check('DD XPBD' not in simulation.modifiers, 'GPU backend adds no Geometry Nodes modifier')
run_frames(scene, 60)
check(seam_gap(simulation) < .03, 'sewn seams close on the body')
run_frames(scene, 60)
check(stretch(simulation) < 1.15, 'cloth stretch stays in range')
coordinates = points(simulation)
held_vertex = int(np.argmax(coordinates[:, 2]))
held_start = coordinates[held_vertex].copy()
xpbd.hold(simulation, [held_vertex])
run_frames(scene, 10)
check(math.dist(points(simulation)[held_vertex], held_start) < 1e-4, 'held vertex stays pinned')
grab = xpbd_live.ClothGrab(simulation)
partner = np.zeros(len(simulation.data.vertices), dtype=np.int32)
simulation.data.attributes['dd_partner'].data.foreach_get('value', partner)
non_seam = np.where(partner == -1)[0]
index = int(non_seam[np.argmin(points(simulation)[non_seam, 2])])
grab_z = float(points(simulation)[index][2])
grab.begin(bpy.context, index, Vector(points(simulation)[index].tolist()), Vector((0, 0, 1)), Vector((0, 1, 0)))
handle = xpbd.input_slot(simulation, 'Grab Handle').value
handle.location.z += 0.1
run_frames(scene, 10)
check(float(points(simulation)[index][2]) - grab_z >= .05, 'grabbed cloth rises with the handle')
grab.release(bpy.context)
run_frames(scene, 30)
_, open_pairs = garment.keep_drape(bpy.context)
check(open_pairs == 0, 'Keep welds every seam pair of the GPU drape')
end = scene.frame_end
garment.reset(bpy.context)
check(scene.frame_end == end and scene.render.fps == 30, 'Reset restores the timeline')

soft = dict(weight_gsm=150, stretch_warp=100, stretch_weft=90, stretch_bias=80, bend_warp=1.5, bend_weft=1.5, bend_bias=1.5)
stiff = dict(weight_gsm=150, stretch_warp=100, stretch_weft=90, stretch_bias=80, bend_warp=4.0, bend_weft=4.0, bend_bias=4.0)
doc = Document.loads(scene.dd_document)
doc.custom_presets['MeasuredSoft'] = measured_to_preset(soft)
doc.custom_presets['MeasuredStiff'] = measured_to_preset(stiff)
for pattern in doc.patterns:
    pattern.preset = 'MeasuredSoft'
scene.dd_document = doc.dumps()

scene.dd_solver = 'NODES'
raised = False
try:
    garment.compile_garment(bpy.context)
except ValueError as exc:
    raised = str(exc) == 'Measured fabrics need the GPU solver. Choose Solver: GPU in Garment & View.'
check(raised, 'measured fabric rejects the NODES solver')
scene.dd_solver = 'GPU'
scene.dd_self_collision = False

volumes = {}
for name, preset in (('MeasuredSoft', soft), ('MeasuredStiff', stiff)):
    doc = Document.loads(scene.dd_document)
    for pattern in doc.patterns:
        pattern.preset = name
    scene.dd_document = doc.dumps()
    garment.reset(bpy.context)
    simulation = garment.compile_garment(bpy.context)
    check(simulation.get('dd_solver') == 'GPU', f'{name} compiles on the GPU backend')
    start_pose = points(simulation).copy()
    run_frames(scene, 60)
    z60 = points(simulation)[:, 2].mean()
    run_frames(scene, 30)
    check(simulation['dd_steps'] == 90, 'every frame steps the solver once')
    check(z60 - points(simulation)[:, 2].mean() < .05, 'draped garment stays on the body')
    coordinates = points(simulation)
    volumes[name] = float((coordinates.max(axis=0) - coordinates.min(axis=0)).prod())
check(volumes['MeasuredStiff'] > volumes['MeasuredSoft'],
      f'stiffer measured fabric drapes larger ({volumes["MeasuredStiff"]:.5f} > {volumes["MeasuredSoft"]:.5f})')
xpbd.hold(simulation, [0])
run_frames(scene, 5)
scene.frame_set(scene.frame_start)
check(np.abs(points(simulation) - start_pose).max() < 1e-5, 'start frame restores the start pose after a pin')
garment.reset(bpy.context)
devil_designer.unregister()
print('ALL GPU CHECKS PASSED', flush=True)
