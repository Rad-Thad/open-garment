mod common;

use common::{run_frames, vertex};
use garment_solver::engine::{Solver, SolverParams};

#[test]
fn seam() {
    let n = 20usize;
    let spacing = 0.01f32;
    let gap = 0.10f32;
    let square = n * n;
    let count = 2 * square;
    let mut positions = Vec::with_capacity(count * 3);
    let mut pairs = Vec::new();
    let mut rest = Vec::new();
    let index_a = |i: usize, j: usize| i * n + j;
    let index_b = |i: usize, j: usize| square + i * n + j;
    for i in 0..n {
        for j in 0..n {
            positions.extend_from_slice(&[j as f32 * spacing, i as f32 * spacing, 0.0]);
        }
    }
    let offset_x = (n - 1) as f32 * spacing + gap;
    for i in 0..n {
        for j in 0..n {
            positions.extend_from_slice(&[j as f32 * spacing + offset_x, i as f32 * spacing, 0.0]);
        }
    }
    for square_idx in 0..2 {
        let base = square * square_idx;
        for i in 0..n {
            for j in 0..n {
                let a = base + i * n + j;
                if j + 1 < n {
                    pairs.extend_from_slice(&[a as u32, (a + 1) as u32]);
                    rest.push(spacing);
                }
                if i + 1 < n {
                    pairs.extend_from_slice(&[a as u32, (a + n) as u32]);
                    rest.push(spacing);
                }
                if i + 1 < n && j + 1 < n {
                    pairs.extend_from_slice(&[a as u32, (a + n + 1) as u32]);
                    rest.push(spacing * std::f32::consts::SQRT_2);
                    pairs.extend_from_slice(&[(a + 1) as u32, (a + n) as u32]);
                    rest.push(spacing * std::f32::consts::SQRT_2);
                }
            }
        }
    }
    let mut seam_pairs = Vec::new();
    for i in 0..n {
        let a = index_a(i, n - 1);
        let b = index_b(i, 0);
        pairs.extend_from_slice(&[a as u32, b as u32]);
        rest.push(0.0);
        seam_pairs.push((a, b));
    }
    let m = rest.len();
    let mut rest_start = rest.clone();
    for (a, b) in &seam_pairs {
        let idx = pairs
            .chunks_exact(2)
            .position(|p| p[0] == *a as u32 && p[1] == *b as u32)
            .unwrap();
        rest_start[idx] = gap;
    }
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, 0.0];
    params.frame_time = 1.0 / 30.0;
    params.seam_seconds = 0.8;
    params.self_thickness = 0.0;
    params.max_step = 0.02;
    let mut solver = Solver::new(
        &positions,
        &vec![1.0; count],
        &pairs,
        &rest,
        &rest_start,
        &vec![1e-9; m],
        &[],
        &positions.clone(),
        &vec![-1; count],
        &vec![1.0; count],
        &params,
            &[],
            &[],
            &[],
    )
    .unwrap();
    run_frames(&mut solver, 60, 12, 2);
    let pos = solver.positions();
    let mut max_gap = 0.0f32;
    for (a, b) in &seam_pairs {
        let pa = vertex(&pos, *a);
        let pb = vertex(&pos, *b);
        let d = ((pa[0] - pb[0]).powi(2) + (pa[1] - pb[1]).powi(2) + (pa[2] - pb[2]).powi(2)).sqrt();
        max_gap = max_gap.max(d);
    }
    assert!(max_gap < 0.001, "max seam gap {max_gap}");
}
