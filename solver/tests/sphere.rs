mod common;

use common::{grid_cloth, run_frames, uv_sphere, vertex};
use garment_solver::engine::{Solver, SolverParams};

#[test]
fn sphere() {
    let n = 64usize;
    let spacing = 0.01f32;
    let mut cloth = grid_cloth(n, n, spacing, 1e-8);
    let center = ((n - 1) as f32 * spacing) / 2.0;
    for p in cloth.positions.chunks_exact_mut(3) {
        p[0] -= center;
        p[1] -= center;
        p[2] = 0.35;
    }
    cloth.rest_positions = cloth.positions.clone();
    let (sphere_verts, sphere_tris) = uv_sphere(0.3, 40, 24);
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, -9.81];
    params.frame_time = 1.0 / 30.0;
    params.body_margin = 0.003;
    params.body_band = 0.03;
    params.self_thickness = 0.0;
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
    solver.set_collider(&sphere_verts, &sphere_tris);
    run_frames(&mut solver, 120, 12, 2);
    let pos = solver.positions();
    let limit = 0.3 + params.body_margin - 0.002;
    for i in 0..n * n {
        let v = vertex(&pos, i);
        let d = (v[0].powi(2) + v[1].powi(2) + v[2].powi(2)).sqrt();
        assert!(d >= limit, "vertex {i} at {d} inside sphere limit {limit}");
    }
}
