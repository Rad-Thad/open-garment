mod common;

use common::{grid_cloth, run_frames, uv_sphere, vertex};
use garment_solver::engine::{Solver, SolverParams};

fn run(cloth: &common::Cloth, params: &SolverParams) -> Vec<f32> {
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
        params,
        &[],
        &[],
        &[],
    )
    .unwrap();
    let (sphere_verts, sphere_tris) = uv_sphere(0.3, 40, 24);
    solver.set_collider(&sphere_verts, &sphere_tris);
    run_frames(&mut solver, 60, 12, 2);
    solver.positions()
}

#[test]
fn repeat() {
    let n = 32usize;
    let spacing = 0.02f32;
    let mut cloth = grid_cloth(n, n, spacing, 1e-8);
    let center = ((n - 1) as f32 * spacing) / 2.0;
    for p in cloth.positions.chunks_exact_mut(3) {
        p[0] -= center;
        p[1] -= center;
        p[2] = 0.32;
    }
    cloth.rest_positions = cloth.positions.clone();
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, -9.81];
    params.frame_time = 1.0 / 30.0;
    params.self_thickness = 0.0;
    params.body_margin = 0.003;
    params.body_band = 0.03;
    params.max_step = 0.02;
    let a = run(&cloth, &params);
    let b = run(&cloth, &params);
    assert_eq!(a, b);
    let mut max_dz = 0.0f32;
    for i in 0..n * n {
        max_dz = max_dz.max((vertex(&a, i)[2] - 0.32).abs());
    }
    assert!(max_dz > 0.01, "cloth did not move, max |z-0.32| = {max_dz}");
}
