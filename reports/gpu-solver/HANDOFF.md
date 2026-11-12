# Open Garment — revision R3 handoff notes

Source: `gpu-solver` @ `2ce4359` (R3 source commit; this file is recorded in the following documentation commit).
Build: `dist/devil_designer-0.15.1.zip`
SHA-256: `c1df4ce7d3e96ce87a4ca4799a9c286e80753a1d8d4dd32d1160e7f06f23879d`

## Withdrawn builds

Both earlier `0.15.0` builds are withdrawn: R1 (SHA-256 `79bcfdf9…`, GPU solver never stepped) and R2 (SHA-256 `18f6bb2e…`). `0.15.0` also collides with upstream's release number. Details are in `STATUS.md`.

## Test status (accurate)

- Real Blender, full `sh dev.sh --test` on this Mac: `ALL TESTS PASSED!` — 15 background suites, `blender_gpu.py`, GPU GUI `gui_core_workflow` and `gui_live_grab`, and all 19 NODES GUI suites.
- Mocked / no Blender: dead-code and import integrity, unit tests including the meshing regression, Rule Zero comment checks, publication check, `git diff --check`.
- Package: extension validation and the real-Blender load gate pass; every zip member equals its source file (package files from `devil_designer/`, `README.md`, `LICENSE`, `SPEC.md` from the repository root; the manifest is generated).
- **Waived by the owner:** `gui_live_brush` under the GPU solver (Fold spreads to cloth far from the brush). It still runs and passes under NODES.

## Recovery-file incident

Before R2, GUI test Blenders wrote `quit.blend` into the shared system temp folder and overwrote the owner's session recovery file; no recovery source was found. Since R2 every test Blender uses `scratch/blender-temp/`; the system `quit.blend` was unchanged throughout R3.

## Ready for your test

Please test this build. If it passes, reply **PR** or **ship it**; I will run the final checks, create or update the PR, merge it into `master`, and close the session.

Short checklist (installed zip, GPU solver unless stated):

1. Tee starter → Simulate at Standard: cloth moves, seams close, stays on the body.
2. Grab: drag the cloth, release; it falls back.
3. While grabbing press W to pin; Ctrl+click frees a pin; Remove pins in the Grab flyout.
4. Fold and Sweep from the Grab flyout shape the playing drape (known issue on GPU: Fold also moves cloth away from the brush).
5. Draw a rectangle about 3 m wide over a 2 m sphere body: edits stay responsive; right-click the piece → Mesh density shows `Auto … cm`.
6. Keep: produces the mesh.
7. Hoodie starter → Simulate → Keep: the hood opening faces forward and no seam stays open.
8. Edge right-click → Elastic & allowance: both choices apply.
9. Production → Sizes: add `L`, switch to it, back to `Base`.
10. Production → Export patterns: DXF opens at true size; PDF test square measures 50 mm.
