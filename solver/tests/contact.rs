mod common;

use common::{grid_cloth, vertex};
use garment_solver::engine::{Solver, SolverParams};

fn plane_collider() -> (Vec<f32>, Vec<u32>) {
    (
        vec![-1.0, -1.0, 0.0, 3.0, -1.0, 0.0, -1.0, 3.0, 0.0],
        vec![0, 1, 2],
    )
}

fn solver_on_plane(cloth: &common::Cloth) -> Solver {
    let (collider_verts, collider_tris) = plane_collider();
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, 0.0];
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
        &[],
        &[],
        &[],
    )
    .unwrap();
    solver.set_collider(&collider_verts, &collider_tris);
    solver
}

#[test]
fn plane_lifts_every_vertex() {
    let mut cloth = grid_cloth(16, 16, 0.01, 1e-8);
    let original = cloth.positions.clone();
    for p in cloth.positions.chunks_exact_mut(3) {
        p[2] = 0.001;
    }
    cloth.rest_positions = cloth.positions.clone();
    let mut solver = solver_on_plane(&cloth);
    solver.step(1, 1).unwrap();
    let pos = solver.positions();
    for i in 0..256 {
        let v = vertex(&pos, i);
        assert!((v[2] - 0.003).abs() < 1e-5, "vertex {i} z {} not at 0.003", v[2]);
        assert!((v[0] - original[3 * i]).abs() < 1e-6, "vertex {i} x moved");
        assert!((v[1] - original[3 * i + 1]).abs() < 1e-6, "vertex {i} y moved");
    }
}

#[test]
fn tail_vertex_contact_is_its_own() {
    let mut cloth = grid_cloth(16, 16, 0.01, 1e-8);
    for p in cloth.positions.chunks_exact_mut(3) {
        p[2] = 0.02;
    }
    cloth.positions[3 * 255 + 2] = 0.001;
    for (i, pair) in cloth.pairs.chunks_exact(2).enumerate() {
        let a = vertex(&cloth.positions, pair[0] as usize);
        let b = vertex(&cloth.positions, pair[1] as usize);
        let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
        cloth.rest[i] = d;
        cloth.rest_start[i] = d;
    }
    cloth.rest_positions = cloth.positions.clone();
    let mut solver = solver_on_plane(&cloth);
    solver.step(1, 1).unwrap();
    let pos = solver.positions();
    let tail = vertex(&pos, 255);
    assert!((tail[2] - 0.003).abs() < 1e-5, "tail vertex z {} not at 0.003", tail[2]);
    for i in 224..255 {
        let v = vertex(&pos, i);
        assert!((v[2] - 0.02).abs() < 1e-6, "vertex {i} z {} moved", v[2]);
    }
}
