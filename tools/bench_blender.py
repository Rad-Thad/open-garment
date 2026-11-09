import sys
import time
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT))
import devil_designer
from devil_designer import garment

devil_designer.register()
scene = bpy.context.scene
scene.frame_set(scene.frame_start)
if not bpy.ops.devil_designer.demo() == {'FINISHED'}:
    raise SystemExit('Demo creation failed.')

for spacing in (.025, .012, .006):
    scene.dd_spacing = spacing
    simulation = garment.compile_garment(bpy.context)
    scene.frame_set(scene.frame_start)
    solver = getattr(scene, 'dd_solver', None) or 'NODES'
    vertices = len(simulation.data.vertices)
    times = []
    for _ in range(120):
        t0 = time.perf_counter()
        scene.frame_set(scene.frame_current + 1)
        times.append(time.perf_counter() - t0)
    mean_ms = sum(times) / len(times) * 1000.0
    print(f"BENCH solver={solver} spacing={spacing} vertices={vertices} ms_per_frame={mean_ms:.3f}", flush=True)

devil_designer.unregister()
print('BENCH DONE', flush=True)
