mod common;

use common::{grid_cloth, Cloth};
use garment_solver::engine::{Solver, SolverParams};

fn run(cloth: &Cloth, params: &SolverParams, n: usize) -> Vec<f32> {
    let count = n * n;
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
    let mut mask = vec![0u8; count];
    let mut targets = vec![0.0f32; count * 3];
    for j in 0..n {
        let idx = (n - 1) * n + j;
        mask[idx] = 1;
        targets[3 * idx] = j as f32 * 0.02;
        targets[3 * idx + 1] = (n - 1) as f32 * 0.02;
        targets[3 * idx + 2] = 0.0;
    }
    solver.set_pins(&mask, &targets);
    for _ in 0..30 {
        solver.step(12, 2).unwrap();
    }
    solver.positions()
}

#[test]
fn repeat() {
    let n = 32usize;
    let cloth = grid_cloth(n, n, 0.02, 1e-8);
    let mut params = SolverParams::default();
    params.gravity = [0.0, -9.81, 0.0];
    params.frame_time = 1.0 / 30.0;
    params.self_thickness = 0.005;
    params.max_step = 0.02;
    let a = run(&cloth, &params, n);
    let b = run(&cloth, &params, n);
    assert_eq!(a, b);
}
