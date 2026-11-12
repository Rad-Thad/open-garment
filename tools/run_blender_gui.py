import sys
import os
import json
import time
from pathlib import Path
import bpy

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'scratch' / 'solver_site'))
sys.path.insert(0, str(ROOT))
from tools.workspace import prepare_scratch

scratch = prepare_scratch()
temporary = scratch / 'blender-temp'
temporary.mkdir(exist_ok=True)
bpy.context.preferences.filepaths.temporary_directory = str(temporary) + os.sep

display = {}


def normalize_display():
    preferences = bpy.context.preferences
    if 'base' not in display:
        display['base'] = max(.5, preferences.view.ui_scale / preferences.system.ui_scale)
        preferences.view.ui_scale = display['base']
        bpy.app.driver_namespace['dd_gui_base_scale'] = display['base']
        return .05
    window = bpy.context.window_manager.windows[0]
    area = next(item for item in window.screen.areas if item.type == 'VIEW_3D')
    region = next(item for item in area.regions if item.type == 'WINDOW')
    print(f'GUI ENV window={window.width}x{window.height} region={region.width}x{region.height} '
          f'system_scale={preferences.system.ui_scale:.2f} view_scale={preferences.view.ui_scale:.2f}', flush=True)
    return None


if sys.platform == 'darwin':
    bpy.app.timers.register(normalize_display, first_interval=0)

test_name = sys.argv[-1] if len(sys.argv) > 1 and not sys.argv[-1].startswith('-') else "gui_preview"
report_file = scratch / f"{test_name.replace('gui_', '').replace('.py', '')}-report.json"
if "preview" in test_name:
    report_file = scratch / "gui-report.json"
elif "silhouette" in test_name:
    report_file = scratch / "silhouette-gui-report.json"
elif "design" in test_name:
    report_file = scratch / "design-gui-report.json"

if report_file.exists():
    try:
        report_file.unlink()
    except Exception:
        pass

target_file = ROOT / "tests" / (test_name if test_name.endswith(".py") else f"{test_name}.py")
print(f"Loading test: {target_file}")
print(f"Expecting report: {report_file}")

try:
    with open(target_file, "r", encoding="utf-8-sig") as f:
        code = compile(f.read(), str(target_file), "exec")
        exec(code, {"__name__": "__main__", "__file__": str(target_file)})
except Exception:
    import traceback
    traceback.print_exc()
    bpy.ops.wm.quit_blender()
    sys.exit(1)

start_time = time.time()

def watchdog():
    if report_file.exists():
        time.sleep(0.5)
        try:
            data = json.loads(report_file.read_text(encoding="utf-8"))
            if "error" in data:
                print("GUI TEST FAILED:")
                print(data["error"])
            else:
                print(f"GUI TEST PASSED ({len(data.get('passed', []))} checks)")
        except Exception as e:
            print("Error reading report:", e)
        bpy.ops.wm.quit_blender()
        return None
    timeout = 120 if test_name in {'gui_live_grab', 'gui_live_brush', 'gui_core_workflow', 'gui_matrix', 'gui_refit'} else 20
    if time.time() - start_time > timeout:
        print("GUI TEST TIMED OUT waiting for report")
        bpy.ops.wm.quit_blender()
        return None
    return 0.5

bpy.app.timers.register(watchdog, first_interval=1.0)
