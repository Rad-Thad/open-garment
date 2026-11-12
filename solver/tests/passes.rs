mod common;

use common::grid_cloth;
use garment_solver::engine::{Solver, SolverParams};

#[test]
fn full_budget_frame_runs() {
    let cloth = grid_cloth(16, 16, 0.01, 1e-8);
    let mut params = SolverParams::default();
    params.gravity = [0.0, -9.81, 0.0];
    params.frame_time = 1.0 / 30.0;
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
        &[],
        &[],
        &[],
    )
    .unwrap();
    solver.step(64, 32).unwrap();
    assert!(solver.positions().iter().all(|v| v.is_finite()));
}
