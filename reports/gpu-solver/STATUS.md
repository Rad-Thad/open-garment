# Open Garment — GPU solver status

Repo: repository root, branch `gpu-solver`.
Remotes: `origin = Rad-Thad/open-garment`, `upstream = Artbyo3/DEVIL-DESIGNER`.

## Revision R3 (2026-11-12): history cleanup, upstream sync, GPU live tools, GUI suites green on macOS

Build: `dist/devil_designer-0.15.1.zip`, SHA-256 `c1df4ce7d3e96ce87a4ca4799a9c286e80753a1d8d4dd32d1160e7f06f23879d`.

### History and remotes

- `M7-STATUS.md` removed from `gpu-solver` history with `git filter-branch` (old tip `412befd` → new tip `c08cac0`, tree identical, 38 → 32 commits; the six commits that only touched that file were dropped). Commit ids quoted in older sections of this file are pre-rewrite. Force-pushed with lease. `tools/check_public.py` passes.
- GitHub may keep the pre-rewrite commits reachable by direct SHA until its own garbage collection; an earlier purge needs a GitHub Support request by the owner.
- Fork `master` fast-forwarded to upstream `e17d9e0`.

### Upstream merge (`e17d9e0`, merge commit `0e2b3b3`)

Conflicts in `library.py`, `presets.py`, `ui.py`, `xpbd.py`, `xpbd_live.py`, `tools/dev.py`, `tools/screenshot.py`, `tests/test_developer_runner.py`; both sides kept. Integration: `xpbd.current` and `ClothGrab.alive` accept GPU-owned objects, `arm` uses `xpbd.vertex_budget`, every tool resolves Blender through `tools.workspace.blender_path`, Blender temp files go to `scratch/blender-temp`, version `0.15.1`.

### Fixes found during R3

| Commit | Finding | Fix |
|---|---|---|
| `1572b2b` | Blender 5.2.2 `mesh.edges.add` crashes (SIGBUS) on meshes with about 37k edges; reproduced on a plain grid without add-on code. Hit at 0.005 m on both solvers | Solver link edges are appended through BMesh |
| `aba7c00` | `blender_torso` Hoodie left one seam pair open, identically at upstream `e17d9e0`: the hood centre seam ended at the front neck, so both back hood corners met only the back-neck vertex | Hood centre seam sewn on the back edges (right 1 ↔ left 3) |
| `8bdf451` | Fork's per-edge Allowance made the Edge card six controls (`gui_fast_flow` budget 5) | Owner choice: one "Elastic & allowance" submenu row |
| `52bf9f8` | GPU drapes differed run to run: Blender's UV-sphere primitive returns faces in varying order and the contact pass breaks ties by triangle order | Collider triangles are put in canonical order before upload; repeated runs give byte-identical positions |
| `2ce4359` | "stiffer measured fabric drapes larger" compared bounding-box volume, which is not monotonic in bending (limp 0.0504, soft 0.0586, stiff 0.0579, board 0.0395) | Owner-approved: compares mean fold angle (limp 11.4°, soft 8.3°, stiff 5.1°, board 3.4°; soft 8.31–8.50°, stiff 5.11–5.12° across seven triangle orders) |

### GPU parity for upstream live tools (`9ddec16`)

| Tool | GPU behaviour |
|---|---|
| Live pins / unpins | `dd_pinned`, `dd_pin_target` read every frame |
| Grab release | `Grab Release` ≥ 0 ends the pull immediately (upstream's 0.57 s ease-out needs soft pins; not ported) |
| Fold, Sweep | Same brush math as the node graph; a held grab overrides the brush |
| Air damping | Mean of `dd_damping` over the garment |

### GUI suites on macOS

Root cause: the test window opened on the portrait 4K main display at UI scale 2. `tools/dev.py` now opens a 960×540-point window (1920×1080 px; region 1574×954) and `tools/run_blender_gui.py` normalises the view scale so the effective scale is 1. `gui_design` multiplies its own UI-scale steps by that base; the library preview test patches `library.entries` instead of reading the user's library folder. Windows open on OmniWM workspace 8 through a scratch wrapper.

### Results on this Mac (source `2ce4359`)

- Real Blender, full `sh dev.sh --test`: `ALL TESTS PASSED!` — 15 background suites, `blender_gpu.py`, GPU GUI `gui_core_workflow` and `gui_live_grab`, all 19 NODES GUI suites.
- Mocked: dead-code/import integrity and unit tests (including meshing regression) pass.
- Package: extension validation and real-Blender load gate pass; zip members identical to source; wheel `garment_solver-0.3.1-cp311-abi3-macosx_11_0_arm64.whl` included.
- Benchmarks (`tools/bench_blender.py`):

| Solver | Spacing | Vertices | ms/frame | Stretch |
|---|---|---|---|---|
| NODES | 0.025 | 820 | 14.3 | 1.029 |
| NODES | 0.012 | 3342 | 17.8 | 1.006 |
| NODES | 0.006 | 12404 | 35.2 | 1.127 |
| GPU | 0.025 | 820 | 9.1 | 1.123 |
| GPU | 0.012 | 3342 | 27.3 | 1.120 |
| GPU | 0.006 | 12404 | 33.9 | 1.199 |

### Open items

- **GPU Fold spread (owner waiver 2026-11-12, `7c8df41`):** `gui_live_brush` under GPU fails "Fold moves far cloth far less than the cloth under the brush" (GPU gathered 4.7 mm, far 7.1 mm; NODES 3.5 / 1.3 mm). Cloth 4–20 cm from the stroke moves 17–30 mm on GPU vs 1.6–3 mm on NODES, mostly outward from the body. Not caused by nondeterminism, substep pin interpolation or iteration count (2/4/8 give the same spread). The suite is out of the GPU GUI loop and still runs under NODES.
- Stretch 1.199 at 0.006 m on GPU (R2 F10).
- Carried from R2: bending calibration waived; sewing-preview rebuild with Frozen pieces.
- Blender's add-on shader `arrangement_controls.display_shader` exceeds the 128-byte push-constant minimum (warning only on this Mac).

### Owner decisions (2026-11-12)

- Merge upstream first; port all upstream live tools to the GPU solver; version 0.15.1; fast-forward fork `master`; rewrite `gpu-solver` history (force push authorized); GUI windows on workspace 8.
- Edge card: Elastic and per-edge Allowance share one submenu.
- GPU Fold spread tabled; `gui_live_brush` removed from the GPU GUI loop until fixed.
- Measured stiffness check uses mean fold angle.

## Revision R2 (2026-11-11 findings, fixes on top of `8c4d4b3`)

The R1 build `dist/devil_designer-0.15.0.zip` (SHA-256 `79bcfdf9…`) is **withdrawn**: the GPU solver did not step at default density.

### Findings — fixed / open

| Id | Finding | Status |
|---|---|---|
| F1 | GPU solver failed every frame once a frame needed more than about 2,048 compute passes (one pass per colour per iteration); Standard 0.02 m needed 2,484; `advance` hid the error and the cloth never moved | Fixed — one compute pass per substep (solver 0.3.1); new `full_budget_frame_runs` (64 × 32) failed with `GPU device lost` before the change, passes after |
| F2 | R1 tests ran only at 0.025 m / 0.04 m; R1 GPU benchmark rows at 0.012 m and 0.006 m timed failing frames | Fixed — `blender_gpu.py` sweeps 0.04 / 0.02 / 0.01 / 0.005 m (every frame steps, cloth moves; stretch and seams at 0.02 / 0.01 m); `bench_blender.py` exits if the GPU skips a frame. Against the 0.3.0 wheel the sweep fails at 0.02 m (`dd_steps` never set) |
| F3 | Single pass per substep is faster | Confirmed — see benchmarks |
| F4 | `StateOverlay.draw` ran a per-vertex Python loop on every redraw | Fixed — returns early without labels; piece groups cached per simulation (name, vertex count, piece ids, signature) |
| F5 | Panel rebuild dominated by the candidate-to-boundary distance loop in `meshing.py` | Fixed — numpy prefilter, exact original test within 1e-9 of the threshold; output identical (below) |
| F6 | No vertex limit for large "Garment default" pieces | Fixed — `AUTO_VERTEX_BUDGET = 6000`; piece menu shows `Auto … cm` when widened |
| F7 | GUI test Blenders wrote `quit.blend` into the shared system temp folder | Fixed — runner children get `TMPDIR`/`TEMP`/`TMP` = `scratch/tmp/`; GUI runner sets Blender's temporary directory there |
| F8 | CI `Source checks` red on the fork owner's commit identity | Fixed — identity approved (owner decision below) |
| F9 | Local `tools/check_public.py` still fails: nine historical blobs of `M7-STATUS.md` (deleted in R1) contain the absolute repository path and local account name; reachable from `origin/gpu-solver`. Already failing at `8c4d4b3` | **Open — owner decision**: history rewrite (needs explicit authorization), or squash-merge (the file is absent from the tree) and delete the branch, or accept |
| F10 | First valid GPU stretch at 0.006 m in `bench_blender.py` is 1.204 (gate 1.15); R1's 1.087 was measured on a cloth that never moved | **Open** — reported, not addressed in R2 |

### Owner decisions (2026-11-11)

- `devil_designer/meshing.py` may receive the speed-only change in `fabric_mesh` (output identical).
- "Garment default" pieces are auto-coarsened to about 6,000 vertices, at every garment density including Ultra; an explicit piece density bypasses the cap.
- Commit identity `Rad_Thad <thaddeusgrosjr@protonmail.com>` is approved for publication.

### Corrections to R1 text

- The R1 GPU benchmark values at 0.012 m and 0.006 m are **invalid**: the solver failed on every frame, so they time error handling, and the stretch values describe an unmoved cloth.
- The "transient Metal device loss" and "hinges every other iteration so the device loss does not fire" notes had the wrong cause. The real cause was the number of compute passes in one command buffer (about 2,048 on this Mac). The every-other-iteration hinge dispatch is kept unchanged because removing it changed nothing measurable (stretch 1.106 vs 1.107).

### Benchmarks (R2)

Crate (`cargo run --release --example bench -- 224`): `vertices=50176 ms_per_frame=16.738` (R1: 23.459; 16 ms gate still exceeded by 0.7 ms).

Blender (`tools/bench_blender.py`, demo, 120 frames, no GPU frame skipped):

| Solver | 0.025 m | 0.012 m | 0.006 m |
|---|---|---|---|
| NODES ms/frame | 16.651 | 12.423 | 26.867 |
| NODES stretch | 1.009 | 1.005 | 1.005 |
| GPU ms/frame | 8.951 | 25.241 | 32.073 |
| GPU stretch | 1.134 | 1.126 | 1.204 |

GPU `ms_per_frame` at 0.025 m is within the 20 ms regression threshold.

GPU sweep in `blender_gpu.py` on a fresh demo (stretch / seam gap after 120 frames): 0.02 m 1.107 / 6.1 mm; 0.01 m 1.121–1.125 / 1.6–5.0 mm.

Meshing equivalence (old `fabric_mesh` from `8c4d4b3` vs new, 96 meshes: three starters, 20 random polygons, the 3.18 × 1.25 m rectangle, each at 0.04 / 0.02 / 0.01 / 0.005 m): identical output including errors; 38.0 s → 5.9 s. New `large` design added to `tests/test_meshing_regression.py` with a baseline computed before the edit.

Artist scenario probe (2 m sphere body, rectangles 3.18 × 1.25 m and 3.61 × 1.48 m, GPU, Garment default 0.02 m): panels `[6136, 6144]` vertices; one corner edit 211 ms (before: 1,724 ms); simulate 30.0 fps over 6 s, 313 steps, no error (before: 8.5 fps, cloth frozen); state-label groups cached after freezing a piece.

### Verification (R2)

Mocked / no Blender: `tools.check_dead_code` clean; `tools/check_comments.py` zero comments or docstrings (147 Python files); Rust/WGSL comment grep empty; `git diff --check` clean; `tests.test_developer_runner` 5/5 and `tests.test_density` (mocked) pass; `cargo test --release` 9/9 (adds `passes`).

Real Blender, background: `sh dev.sh --test` runs OK through static checks, mocked unit tests, all 14 background suites, the wheel build, `blender_gpu.py`, `gui_core_workflow` (GPU), `gui_preview`, `gui_silhouette`, then stops at `gui_design`. Separately: `blender_gpu.py` **ALL GPU CHECKS PASSED** with the new sweep; `blender_density.py` passed before and after the `meshing.py` edit with the same baseline.

Real Blender, GUI (each suite run on its own, `DD_SOLVER=NODES`, own window on workspace 3): `gui_library` 242, `gui_arrangement` 54, `gui_fit_map` 13, `gui_viewport_sewing` 118, `gui_core_workflow` 159 (also 159 under GPU, including the two new playback-failure checks), `gui_editor_release` 21 checks **pass** (the baseline listed this suite as failing). `gui_preview` 26 passes and leaves the system `quit.blend` untouched; the file is written to `scratch/tmp/`.

Failing GUI suites (the AGENTS rule that every suite must pass is **not met** on this Mac):

| Suite | First failure | Same at `8c4d4b3`? |
|---|---|---|
| `gui_design` | `ellipse tool creates four Bezier anchors` | yes (rerun) |
| `gui_arrangement_display` | `idle, hover and pressed markers differ in grayscale {'IDLE/HOVER': 0.0, …}` | yes (rerun); differs from the mac-baseline text |
| `gui_fast_flow` | `requesting a library load never changes settings before confirmation` | matches mac baseline |
| `gui_avatar_field` | `clicking outside closes only the list` | matches mac baseline |
| `gui_live_grab` | `Grab controls are laid out` | matches mac baseline |
| `gui_matrix` | `target is visible in the 3D view` | matches mac baseline |

None of these is caused by R2. The draw-coverage gate (`gui_fast_flow`) stays unmet.

### Deviation from the R2 plan

- The density sweep in `tests/blender_gpu.py` runs directly after demo creation, not after the Keep check. A kept garment is a collision body for the next drape by default ("Wear over", `layering.collision_bodies`), so a drape after Keep is a layered drape over the kept `DD Garment`: 0.01 m stretch 1.159–1.165 after Keep vs 1.125 fresh and 1.127 after a reset without Keep. The sweep measures single-layer drapes; layered-drape stretch above 1.15 is reported, not gated. The sweep reads `dd_steps` with `.get` so a never-stepped solver fails the named check instead of raising `KeyError`.
- Vertex budget with seams: a large Garment default piece sewn to a small piece keeps its widened interior spacing; only its boundary follows the finer shared sampling (6,136 vertices alone, 6,258 sewn; 0.02 m garment density).

### Not addressed (reported)

- Bending calibration stays waived: at Standard density the measured soft and stiff fabrics drape the same, so `stiffer measured fabric drapes larger` stays at 0.025 m.
- With a Frozen or Strengthened piece the 3D sewing preview rebuilds panels about twice a second (4 fps with two 6,000-vertex pieces).
- The draw-coverage gate (`gui_fast_flow`) remains unmet on this Mac.

## Revision R1 (head reviewed at `bb73ec8`)

### Findings — fixed / open

| Id | Finding | Status |
|---|---|---|
| S1 | `contact_out` stride 28 vs WGSL 32 (last 1/8 share a slot) | Fixed — `CONTACT_STRIDE = 32` |
| S2 | `closest_bary` interior return `(1-v-w, v, w)` (callers read x=weight b, y=c, z=a) | Fixed — `(v, w, 1-v-w)` in WGSL + Rust fixture |
| S3 | Self-contact hash built/queried in pattern space | Fixed — hash and lookup use current positions |
| S4 | Solver tests cannot fail (sphere/fold/repeat/bench) | Fixed — new `contact`/`layers` tests, restored `sphere` bound, `repeat` motion, bench advances `elapsed` |
| S5 | `set_pins` panics on short mask; `positions()` zeros before step; hinge divide by zero | Fixed — length check, cached positions, `denom <= 0` guard |
| S6 | Calibration fixture (solid incline collider + capped bracket) | Rebuilt — collider removed, bracket widened to 0.5×–2.0×; gate still not met (below) |
| B1 | `gpu_backend._clear` not `@persistent` | Fixed — `clear()` decorated |
| B2 | Frame skip `==` and `_last_frames` never cleared | Fixed — monotonic guard + `clear()` |
| B3 | Start pose only in session cache | Fixed — `dd_start_position` mesh attribute |
| B4 | Failed `xpbd.attach` leaves signed object | Fixed — try/except removes new anchors |
| B5 | Measured fabrics still get preset bend links; Strengthen ignores `dd_flex_*`; `prepare()` no invalidate; 10 scene props missing; per-hinge Python loops | Fixed — all six items |
| T1 | `blender_gpu.py` loosened (stretch 1.5, Keep retry, `dd_pin_top`, substring NODES reject) | Fixed — stretch 1.15, single Keep + `open_pairs == 0`, no `dd_pin_top`, exact reject string |
| T2 | GUI suites only under NODES; no GPU benchmark recorded | Fixed — GPU `gui_core_workflow` in `do_tests`, bench records `stretch` |
| C1 | `Source checks` fails on every push (`tools/dev.py` SystemExit at import) | Fixed — `blender_path()` in `tools/workspace.py` |
| C2 | `macos-14` wheel job retired | Fixed — `macos-15` |
| P1 | Export operator + PDF scale/naming/text-mode/non-ASCII | Fixed — bytes writers, point-space PDF, `check()`, `_export_path`, non-ASCII safe |
| P2 | Grades not transformed by flip/rotate/scale/mirror-clone/twin/slash-spread/merge/library; allowances lost on clone/split; cut lag; t=23/24 | Fixed — `map_grades` everywhere, allowance remap, `curves.controls`, `range(25)` |
| P3 | Sizes card base row; choice rows overlap; Units→Preset keeps `measured`; graded outlines via full JSON round-trip | Fixed — `SET_SIZE_` base row, `choices_height`, `drop_measured`, `production.graded_outline` |
| G1 | `gui_design`/`gui_matrix` fail earlier than baseline | Pre-existing — see step 0 |
| H1 | `M7-STATUS.md` in root; stale dist; scratch/pycache/build leftovers | Fixed — moved to `reports/gpu-solver/STATUS.md`, cleanup below |

### Step 0 — `gui_design` / `gui_matrix`

Each suite run 3× at HEAD `bb73ec8`, then once at `550c94e` (M0 fork, app code identical to upstream). Failure text identical at both commits:

- `gui_design` — `Region and click selection … UI scale 2, zoom 300, enclose selects two panels` (then mixed-drag, pivot, 3D arrangement).
- `gui_matrix` — `target is visible in the 3D view`.

Verdict: **pre-existing environment drift, not a fork regression.** No code change. The G1 review note ("ellipse tool creates four Bezier anchors" / "dragging the top bar moves the panel") was stale scratch-report data; those specific tests now pass. **Corrected in R2:** this was wrong for the ellipse check. `gui_design` fails first on `ellipse tool creates four Bezier anchors` at `8c4d4b3` and at the R2 source (R1 HANDOFF already recorded that `dev.sh --test` stops there).

### Calibration gate — NOT met, owner waiver stands

`python3 tools/calibrate_hinge.py` (collider removed, bracket 0.5×–2.0×):

| Round | HINGE_SCALE | bend 1.5 cm | bend 2.5 cm | bend 4.0 cm |
|---|---|---|---|---|
| 1 (5.0 mm) | 3.0 | ratio 0.500 | 0.500 | 0.500 |
| 1 (2.5 mm) | 3.0 | ratio 0.500 | 0.500 | 0.500 |
| 2 (5.0 mm) | 24.0 | ratio 0.500 | 0.500 | 0.500 |
| 2 (2.5 mm) | 24.0 | ratio 0.500 | 0.500 | 0.500 |

All six cells sit at the low bracket (0.500): the strip bends past 41.5° at the shortest tested overhang, and an 8× `HINGE_SCALE` change leaves the ratio unchanged. The hinge compliance maps to a very soft constraint (`alpha = compliance * h2_inv` is large), so the cantilever is effectively free-falling and the bisection carries no signal. Per the plan rule the gate is **not met**; the owner's waiver stands. `HINGE_SCALE` is kept at `3.0` (the value the verified GPU measured-fabric test was built against).

### Benchmarks

Crate (`cargo run --release --example bench -- 224`): `vertices=50176 ms_per_frame=23.459` — **exceeds** the 16 ms gate.

Blender (`tools/bench_blender.py`, demo, 120 frames):

| Solver | 0.025 m | 0.012 m | 0.006 m |
|---|---|---|---|
| NODES ms/frame | 16.723 | 12.051 | 29.134 |
| NODES stretch | 1.009 | 1.005 | 1.005 |
| GPU ms/frame | 54.341 | 204.978 (invalid, see R2) | 218.216 (invalid, see R2) |
| GPU stretch | 1.131 | 1.108 (invalid) | 1.087 (invalid) |

Gates: stretch **met** (all ≤ 1.15). Speed **not met** — GPU is ~3×–17× slower than NODES and far above `NODES/5` and the 33 ms cap at 0.006 m. Root cause is the per-colour Gauss-Seidel dispatch (`iterations × colours` compute passes per substep) plus `ITERATIONS = 8` to meet stretch. The plan's Jacobi remedy (one vertex-centric dispatch per iteration, doubled iterations) diverges for the stiff `hang` constraints (compliance 1e-8) without an under-relaxation term the plan does not specify, so it is not applied. The stretch-over-speed priority is honoured (scaled iterations kept) and both numbers are reported.

R2 correction: "stretch met" above relied on the invalid 0.012 m and 0.006 m GPU rows; the first valid 0.006 m value is 1.204 (F10). The R1 Verification counts below are R1 numbers; R2 counts are in "Verification (R2)".

### Verification

Mocked (no Blender): `tools.check_dead_code` clean; `unittest discover -s tests -p 'test_*.py'` **482 pass, 24 skipped** (includes new export + grade-edit cases); `cargo test --release` **8/8** (contact ×2, hang, hinge, layers, repeat, seam, sphere); `grep -rnE '//|/\*' solver/src solver/tests solver/examples` empty.

Real Blender: `blender_gpu.py` **ALL GPU CHECKS PASSED** (restored 1.15 stretch, Keep `open_pairs == 0`, exact NODES rejection, no `dd_pin_top`, 90-step count, stay-on-body, start-pose restore); `blender_production.py` **ALL PRODUCTION CHECKS PASSED**; `gui_core_workflow` **PASSED under both NODES and GPU**.

GPU contact-path proof: with S1/S2/S3 temporarily reverted, `contact` (S1/S2) and `layers` (S3) fail; the `sphere` penetration was additionally traced to an inside-out `uv_sphere` fixture (winding corrected in the test/bench fixture).

Known-failing GUI suites on this Mac (pre-existing, from `reports/mac-baseline/STATUS.md`): `gui_design`, `gui_fast_flow`, `gui_arrangement_display`, `gui_avatar_field`, `gui_editor_release`, `gui_live_grab`, `gui_matrix`. Passing: `gui_preview`, `gui_silhouette`, `gui_library`, `gui_arrangement`, `gui_fit_map`, `gui_viewport_sewing`, `gui_core_workflow`. The draw-coverage gate (`gui_fast_flow`) remains unmet on this Mac and is stated as such.

### Accepted limitations

- Calibration gate: not met; owner waiver stands (see table).
- Crate and Blender speed gates: not met (see table); stretch gates met in R1 only on cloth that, at 0.012 m and 0.006 m, never moved. R2 measures 1.204 at 0.006 m (F10).
- Superseded in R2: the GPU solver failures attributed here to "a transient Metal device loss" were the per-command-buffer compute pass limit; `advance` now reports the failure instead of hiding it.
- The measured-fabric volume check (`stiffer drapes larger`) is non-deterministic under self-collision (the GPU hash chain uses atomics), so the measured loop disables `dd_self_collision` for that section; its assertions (step count, stay-on-body, bend differentiation) exercise body contact and bending, not self-collision.
