# Open Garment — revision R2 handoff notes

Source: `gpu-solver` @ `3bca266` (R2 source commit; this file is recorded in the following documentation commit).
Build: `dist/devil_designer-0.15.0.zip`
SHA-256: `18f6bb2e90a405cc4cf7906d51ab1b86733b56182ab28bbcfa952f958f51995c`

## Withdrawn build

The R1 build with SHA-256 `79bcfdf953720cb526427953b3149b6da05e973b6a652e75e3cde3b20508793e` is withdrawn. Its GPU solver failed on every frame at Standard, Fine and Ultra density, so the cloth never moved, and the failure was hidden instead of shown. Details and the fixes are in `STATUS.md`, section "Revision R2".

## Test status (accurate)

- Mocked / no Blender: dead-code, comment (Rule Zero token + AST), Rust/WGSL comment, and `git diff --check` checks clean; `cargo test --release` 9/9.
- Real Blender, background: all 14 background suites, the wheel build, `blender_gpu.py` (now covers 0.04 / 0.02 / 0.01 / 0.005 m), extension validation and the real-Blender package load gate pass.
- Real Blender, GUI: `gui_core_workflow` passes under GPU and NODES; `gui_preview`, `gui_silhouette`, `gui_library`, `gui_arrangement`, `gui_fit_map`, `gui_viewport_sewing`, `gui_editor_release` pass.
- **Not met:** the AGENTS rule that every suite passes. `gui_design`, `gui_arrangement_display`, `gui_fast_flow`, `gui_avatar_field`, `gui_live_grab`, `gui_matrix` fail on this Mac with the same first failure at `8c4d4b3` (or the recorded Mac baseline). This build does not cause them. The draw-coverage gate (`gui_fast_flow`) stays unmet.

## Open blocker (owner decision)

`python3 tools/check_public.py` fails locally on nine historical versions of `M7-STATUS.md`. That file was deleted from the tree in R1, but its old versions are still in the Git history of `origin/gpu-solver` and contain the absolute repository path and the local account name. This was already failing at `8c4d4b3`. Options:

1. Rewrite the branch history to drop those versions (needs your explicit authorization; requires a force push).
2. Squash-merge into `master` (the file is not in the current tree) and then delete the `gpu-solver` branch, so no published ref reaches those versions.
3. Accept it as a known limitation.

The commit identity finding is resolved: `Rad_Thad <thaddeusgrosjr@protonmail.com>` is approved in `tools/check_public.py`.

## Ready for your test

Please test this build. If it passes, reply **PR** or **ship it**; I will run the final checks, create/update the PR, merge into `master`, and close the session.

Short checklist (installed zip, GPU solver):

1. Tee starter → Simulate at Standard: cloth moves, seams close, stays on the body.
2. Mesh density Fine → Simulate: same.
3. Draw a rectangle about 3 m wide over a 2 m sphere body: edits stay responsive; right-click the piece → Mesh density shows `Auto … cm`.
4. Pin a point, rewind to start: garment returns to its start pose.
5. Keep: produces the mesh.
6. Right-click piece: seam allowance chips.
7. Production → Sizes: add `L`, switch to it, switch back to `Base`.
8. Production → Export patterns: DXF opens at true size; PDF test square measures 50 mm.
