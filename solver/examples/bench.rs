use garment_solver::engine::{Solver, SolverParams};
use std::time::Instant;

fn grid_cloth(n: usize, spacing: f32, compliance: f32) -> (Vec<f32>, Vec<f32>, Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>, Vec<f32>, Vec<i32>, Vec<f32>) {
    let count = n * n;
    let index = |i: usize, j: usize| i * n + j;
    let mut positions = Vec::with_capacity(count * 3);
    for i in 0..n {
        for j in 0..n {
            positions.extend_from_slice(&[j as f32 * spacing, i as f32 * spacing, 0.0]);
        }
    }
    let mut pairs = Vec::new();
    let mut rest = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if j + 1 < n {
                pairs.extend_from_slice(&[index(i, j) as u32, index(i, j + 1) as u32]);
                rest.push(spacing);
            }
            if i + 1 < n {
                pairs.extend_from_slice(&[index(i, j) as u32, index(i + 1, j) as u32]);
                rest.push(spacing);
            }
            if i + 1 < n && j + 1 < n {
                pairs.extend_from_slice(&[index(i, j) as u32, index(i + 1, j + 1) as u32]);
                rest.push(spacing * std::f32::consts::SQRT_2);
                pairs.extend_from_slice(&[index(i, j + 1) as u32, index(i + 1, j) as u32]);
                rest.push(spacing * std::f32::consts::SQRT_2);
            }
        }
    }
    let mut triangles = Vec::new();
    for i in 0..n - 1 {
        for j in 0..n - 1 {
            triangles.extend_from_slice(&[
                index(i, j) as u32,
                index(i, j + 1) as u32,
                index(i + 1, j) as u32,
            ]);
            triangles.extend_from_slice(&[
                index(i, j + 1) as u32,
                index(i + 1, j + 1) as u32,
                index(i + 1, j) as u32,
            ]);
        }
    }
    let m = rest.len();
    (
        positions.clone(),
        vec![1.0; count],
        pairs,
        rest.clone(),
        rest.clone(),
        vec![compliance; m],
        triangles,
        positions,
        vec![-1; count],
        vec![1.0; count],
    )
}

fn uv_sphere(radius: f32, segments: usize, rings: usize) -> (Vec<f32>, Vec<u32>) {
    let mut vertices = Vec::new();
    for ring in 0..=rings {
        let theta = std::f32::consts::PI * ring as f32 / rings as f32;
        let st = theta.sin();
        let ct = theta.cos();
        for seg in 0..=segments {
            let phi = 2.0 * std::f32::consts::PI * seg as f32 / segments as f32;
            vertices.extend_from_slice(&[radius * st * phi.cos(), radius * st * phi.sin(), radius * ct]);
        }
    }
    let mut triangles = Vec::new();
    let stride = segments + 1;
    for ring in 0..rings {
        for seg in 0..segments {
            let a = (ring * stride + seg) as u32;
            let b = (ring * stride + seg + 1) as u32;
            let c = ((ring + 1) * stride + seg) as u32;
            let d = ((ring + 1) * stride + seg + 1) as u32;
            triangles.extend_from_slice(&[a, b, c]);
            triangles.extend_from_slice(&[b, d, c]);
        }
    }
    (vertices, triangles)
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(224);
    let spacing = 0.02f32;
    let (mut positions, inverse_mass, pairs, rest, rest_start, compliance, triangles, rest_positions, partner, contact_weight) =
        grid_cloth(n, spacing, 1e-8);
    let center = ((n - 1) as f32 * spacing) / 2.0;
    for p in positions.chunks_exact_mut(3) {
        p[0] -= center;
        p[1] -= center;
        p[2] = 0.6;
    }
    let (sphere_verts, sphere_tris) = uv_sphere(0.3, 40, 24);
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, -9.81];
    params.frame_time = 1.0 / 30.0;
    params.self_thickness = 0.01;
    params.body_margin = 0.003;
    params.body_band = 0.03;
    params.max_step = 0.02;
    let mut solver = Solver::new(
        &positions,
        &inverse_mass,
        &pairs,
        &rest,
        &rest_start,
        &compliance,
        &triangles,
        &rest_positions,
        &partner,
        &contact_weight,
        &params,
            &[],
            &[],
            &[],
    )
    .unwrap();
    solver.set_collider(&sphere_verts, &sphere_tris);
    let mut total = 0.0f64;
    let frames = 120u32;
    for _ in 0..frames {
        let t0 = Instant::now();
        solver.step(12, 2).unwrap();
        total += t0.elapsed().as_secs_f64();
    }
    let mean_ms = total / frames as f64 * 1000.0;
    println!("vertices={} ms_per_frame={mean_ms:.3}", n * n);
}
