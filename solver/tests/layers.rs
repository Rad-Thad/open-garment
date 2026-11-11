mod common;

use common::{grid_cloth, merge, run_frames, vertex};
use garment_solver::engine::{Solver, SolverParams};

#[test]
fn layers() {
    let mut lower = grid_cloth(24, 24, 0.01, 1e-8);
    for p in lower.positions.chunks_exact_mut(3) {
        p[0] -= 0.02;
        p[1] -= 0.02;
    }
    lower.rest_positions = lower.positions.clone();
    for w in lower.inverse_mass.iter_mut() {
        *w = 0.0;
    }
    let mut upper = grid_cloth(20, 20, 0.01, 1e-8);
    for p in upper.positions.chunks_exact_mut(3) {
        p[0] += 0.005;
        p[1] += 0.005;
        p[2] = 0.015;
    }
    upper.rest_positions = upper.positions.clone();
    for p in upper.rest_positions.chunks_exact_mut(3) {
        p[0] += 10.0;
    }
    let cloth = merge(lower, upper);
    let mut params = SolverParams::default();
    params.self_thickness = 0.01;
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
    run_frames(&mut solver, 90, 12, 2);
    let pos = solver.positions();
    assert!(pos.iter().all(|v| v.is_finite()), "positions diverged");
    let lower_count = 24 * 24;
    let upper_count = 20 * 20;
    let mut max_upper_z = 0.0f32;
    for i in lower_count..lower_count + upper_count {
        let v = vertex(&pos, i);
        assert!(v[2] >= 0.005, "upper vertex {i} z {} below 0.005", v[2]);
        max_upper_z = max_upper_z.max(v[2]);
    }
    assert!(max_upper_z < 0.015, "upper sheet rose above start, max z {max_upper_z}");
}
