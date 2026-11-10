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
simulation = garment.compile_garment(bpy.context)
check(simulation.get('dd_solver') == 'GPU', 'simulation object is owned by the GPU backend')
check('DD XPBD' not in simulation.modifiers, 'GPU backend adds no Geometry Nodes modifier')
run_frames(scene, 60)
check(seam_gap(simulation) < .03, 'sewn seams close on the body')
check(stretch(simulation) < 1.5, 'cloth stretch stays in range')
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
grab.release()
run_frames(scene, 30)
result = None
for _ in range(4):
    try:
        result = garment.keep_drape(bpy.context)
        break
    except ValueError:
        run_frames(scene, 30)
check(result is not None and result[1] <= 5, 'Keep welds the GPU drape')
end = scene.frame_end
garment.reset(bpy.context)
check(scene.frame_end == end and scene.render.fps == 30, 'Reset restores the timeline')
devil_designer.unregister()
print('ALL GPU CHECKS PASSED', flush=True)
