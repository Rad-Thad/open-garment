# Mac baseline — 2026-11-09

Base commit `1e4e09e53cbbb85588a5ec06386434543e23d86c` (upstream v0.15.0), Blender 5.2.2 LTS arm64, Apple Silicon.

Command: `sh dev.sh --test` (plus each GUI suite re-run individually to complete the matrix, since `do_tests` stops at the first failure).

## Static & mocked headless

| Suite | Result |
|---|---|
| Static import & dead-code integrity | PASS |
| Mocked headless unit tests (`test_*.py`) | PASS |

## Real-Blender background / load smoke

| Suite | Result |
|---|---|
| blender_smoke.py | PASS |
| blender_garment.py | PASS |
| blender_advanced.py | PASS |
| blender_silhouette.py | PASS |
| blender_density.py | PASS |
| blender_arrangement.py | PASS |
| blender_tack_binding.py | PASS |
| blender_tracing.py | PASS |
| blender_layering.py | PASS |
| blender_piece_states.py | PASS |
| blender_sewing_preview.py | PASS |
| blender_xpbd.py | PASS |
| blender_matrix.py | PASS |

## In-engine GUI suites

| Suite | Result |
|---|---|
| gui_preview | PASS |
| gui_silhouette | PASS |
| gui_library | PASS |
| gui_arrangement | PASS |
| gui_fit_map | PASS |
| gui_viewport_sewing | PASS |
| gui_core_workflow | PASS |

## Known-failing set (macOS; pre-existing, not ported)

These suites fail on macOS at this base commit. The "no regression" gate for later milestones is: no suite outside this set fails.

| Suite | Failure text |
|---|---|
| gui_design | region/click selection: `UI scale 2, zoom 300, enclose selects two panels`; `mixed group drag preserves selection on press`; `UI scale 2, zoom 300, pivot is labeled Free`; `a facing arrangement point is available outside the 2D panel` |
| gui_fast_flow | `requesting a library load never changes settings before confirmation` |
| gui_arrangement_display | `a point on the body edge flattens into a sliver instead of facing the camera {'front': (33.0, 34.0), 'side': (11.0, 22.0)}` |
| gui_avatar_field | `clicking outside closes only the list` |
| gui_editor_release | `chest_point()` `StopIteration` (no facing chest point inside the region) |
| gui_live_grab | `Grab controls are laid out` (`{'RADIUS_MINUS', 'RADIUS_PLUS'} <= set(strip)` false) |
| gui_matrix | `target is visible in the 3D view` |

## Geometry-Nodes engine timings (baseline)

`tools/bench_blender.py`, demo garment, 120 frames, mean ms/frame.

```
BENCH solver=NODES spacing=0.025 vertices=820 ms_per_frame=17.074
BENCH solver=NODES spacing=0.012 vertices=3342 ms_per_frame=11.648
BENCH solver=NODES spacing=0.006 vertices=12934 ms_per_frame=28.789
```

## R1 re-check (2026-11-12)

gui_design and gui_matrix were re-run 3× at `bb73ec8` and once at `550c94e`; the first-failure text is identical to the rows above, so both remain pre-existing environment drift on this Mac, not a fork regression.
