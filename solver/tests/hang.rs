mod common;

use common::{grid_cloth, run_frames, vertex};
use garment_solver::engine::{Solver, SolverParams};

#[test]
fn hang() {
    let n = 64usize;
    let spacing = 0.01f32;
    let cloth = grid_cloth(n, n, spacing, 1e-8);
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
    )
    .unwrap();
    let count = n * n;
    let mut mask = vec![0u8; count];
    let mut targets = vec![0.0f32; count * 3];
    for j in 0..n {
        let idx = (n - 1) * n + j;
        mask[idx] = 1;
        targets[3 * idx] = j as f32 * spacing;
        targets[3 * idx + 1] = (n - 1) as f32 * spacing;
        targets[3 * idx + 2] = 0.0;
    }
    solver.set_pins(&mask, &targets);
    run_frames(&mut solver, 90, 12, 16);
    let pos = solver.positions();
    assert!(pos.iter().all(|v| v.is_finite()));
    let mut ratios = Vec::new();
    for (i, pair) in cloth.pairs.chunks_exact(2).enumerate() {
        if cloth.rest[i] != spacing {
            continue;
        }
        let a = vertex(&pos, pair[0] as usize);
        let b = vertex(&pos, pair[1] as usize);
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        ratios.push(d / spacing);
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((ratios.len() - 1) as f32 * 0.99) as usize;
    assert!(ratios[idx] <= 1.05, "99th percentile stretch {}", ratios[idx]);
}
