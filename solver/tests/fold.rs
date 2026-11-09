mod common;

use common::{box_collider, closest_bary, grid_cloth, run_frames, tri_point, vertex};
use garment_solver::engine::{Solver, SolverParams};

#[test]
fn fold() {
    let nx = 100usize;
    let ny = 20usize;
    let spacing = 0.01f32;
    let mut cloth = grid_cloth(nx, ny, spacing, 1e-8);
    for p in cloth.positions.chunks_exact_mut(3) {
        p[2] = 0.35;
    }
    cloth.rest_positions = cloth.positions.clone();
    let (mut box_verts, box_tris) = box_collider([0.01, 0.15, 0.15]);
    for p in box_verts.chunks_exact_mut(3) {
        p[0] += (nx - 1) as f32 * spacing / 2.0;
        p[1] += (ny - 1) as f32 * spacing / 2.0;
        p[2] += 0.15;
    }
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, -9.81];
    params.frame_time = 1.0 / 30.0;
    params.self_thickness = 0.01;
    params.body_margin = 0.003;
    params.body_band = 0.03;
    params.max_step = 0.02;
    let mut solver = Solver::new(
        &cloth.positions,
        &cloth.inverse_mass,
        &cloth.pairs,
        &cloth.rest,
        &cloth.rest_start,
        &cloth.compliance,
        &cloth.triangles,
        &cloth.rest_positions,
        &cloth.partner,
        &cloth.contact_weight,
        &params,
    )
    .unwrap();
    solver.set_collider(&box_verts, &box_tris);
    run_frames(&mut solver, 200, 12, 2);
    let pos = solver.positions();
    assert!(pos.iter().all(|v| v.is_finite()), "positions diverged");
    let count = nx * ny;
    let threshold = 0.5 * params.self_thickness;
    for i in 0..count {
        let v = vertex(&pos, i);
        let rv = vertex(&cloth.rest_positions, i);
        for tri in cloth.triangles.chunks_exact(3) {
            let a = tri[0] as usize;
            let b = tri[1] as usize;
            let c = tri[2] as usize;
            if a == i || b == i || c == i {
                continue;
            }
            let ra = vertex(&cloth.rest_positions, a);
            let rb = vertex(&cloth.rest_positions, b);
            let rc = vertex(&cloth.rest_positions, c);
            let bary = closest_bary(rv, ra, rb, rc);
            let rest_q = tri_point(bary, ra, rb, rc);
            let dr = ((rv[0] - rest_q[0]).powi(2) + (rv[1] - rest_q[1]).powi(2) + (rv[2] - rest_q[2]).powi(2)).sqrt();
            if dr <= 4.0 * params.self_thickness {
                continue;
            }
            let pa = vertex(&pos, a);
            let pb = vertex(&pos, b);
            let pc = vertex(&pos, c);
            let d = {
                let q = tri_point(closest_bary(v, pa, pb, pc), pa, pb, pc);
                ((v[0] - q[0]).powi(2) + (v[1] - q[1]).powi(2) + (v[2] - q[2]).powi(2)).sqrt()
            };
            assert!(d >= threshold, "vertex {i} penetrates non-adjacent triangle by {d}");
        }
    }
}
