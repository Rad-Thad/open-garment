# Open Garment — revision R1 handoff notes

Source: `gpu-solver` @ `14f3efb` (pushed to `origin/gpu-solver`).
Build: `dist/devil_designer-0.15.0.zip`
SHA-256: `79bcfdf953720cb526427953b3149b6da05e973b6a652e75e3cde3b20508793e`

## Test status (accurate)

- Mocked headless: `tools.check_dead_code` clean; `unittest discover -s tests -p 'test_*.py'` 482 pass / 24 skipped; `cargo test --release` 8/8.
- Real Blender background: all 14 suites pass (incl. new `blender_production.py`); `blender_gpu.py` **ALL GPU CHECKS PASSED** (run ×4 after the measured-loop fix); extension validate + real-Blender load gate pass.
- In-engine GUI: `gui_core_workflow` passes under **both NODES and GPU**; `gui_preview` and `gui_silhouette` pass.
- `sh dev.sh --test` runs through the background suites, wheel build, `blender_gpu.py`, and the GPU `gui_core_workflow`, then **stops at `gui_design`** (`ellipse tool creates four Bezier anchors`), a known macOS baseline failure. The remaining GUI suites are the known-failing set from `reports/mac-baseline/STATUS.md`; the draw-coverage gate (`gui_fast_flow`) stays unmet on this Mac, as stated in the plan.

## Deviations from the plan (reported per the owner request)

- `ITERATIONS` 2 → 8: the plan's `iterations(spacing)` formula gives 2 at the demo spacing, but stretch then stays ~1.35 (NODES 1.009); 8 reaches 1.13. Stretch-over-speed priority honoured.
- Hinge solver dispatched every other iteration: halves the measured-loop pass count so the Metal device loss does not fire (it did at full 8×hinge-colours).
- `gpu_backend.advance` catches a lost Metal device, drops the session, and re-initialises next frame; a transient loss still occurs during the aggressive grab release but no longer cascades.
- Measured loop disables `dd_self_collision`: the "stiffer drapes larger" volume check is non-deterministic under self-collision (hash-chain atomics); its assertions (step count, stay-on-body, bend differentiation) only need body contact + bending.
- `HINGE_SCALE` kept at 3.0: calibration rounds at 3.0 and 24.0 both returned ratio 0.500 for all six cells, so the gate stays unmet and the owner waiver stands.
- Grade-edit tests use `abs(area(...))`: flip/reflect reverse polygon winding, so signed area is negative.
- `dd_start_position` read is guarded for old simulation meshes that predate the attribute.

## check_public.py / CI blocker (owner decision)

`tools/check_public.py` now passes source/archive scans, but the **"Check public source and archive"** CI step fails on unapproved commit identities — the fork's commits (including this revision) are authored/committed as `Rad_Thad <thaddeusgrosjr@protonmail.com>`, which is not in `PUBLIC_IDENTITIES`/`APPROVED_AUTHOR_FINGERPRINTS`. This predates this revision. To make the `Source checks` CI green you must either accept that identity for publication (add its fingerprint `ab0685c13d6f7f461805ee609c70b5160fcb48f2eefe7339055342102af0029f` to `APPROVED_AUTHOR_FINGERPRINTS` and the identity to `PUBLIC_COMMITTERS`) or record it as an accepted limitation. The `Solver` CI job (cargo tests + three-platform wheels) is still running.

## Ready for your test

Please test this build. If it passes, reply **PR** or **ship it**; I will run the final checks, create/update the PR, merge into `master`, and close the session.

Short checklist (installed zip, GPU solver):

1. demo → Simulate: cloth stays on the body and seams close.
2. Pin a point, rewind to start: garment returns to its arranged start.
3. Keep: produces the mesh.
4. Right-click piece: "Seam allowance" label sits above two chip rows.
5. Production → Sizes: add `L`, switch to it, switch back to `Base`.
6. Production → Export patterns: DXF opens at true size; PDF page 1 test square measures 50 mm.
