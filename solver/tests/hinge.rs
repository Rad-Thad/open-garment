mod common;

use garment_solver::engine::{Solver, SolverParams};

fn dihedral(positions: &[f32]) -> f32 {
    let a = [positions[0], positions[1], positions[2]];
    let b = [positions[3], positions[4], positions[5]];
    let c = [positions[6], positions[7], positions[8]];
    let d = [positions[9], positions[10], positions[11]];
    let e = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let elen = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
    let n1 = [
        e[1] * (c[2] - a[2]) - e[2] * (c[1] - a[1]),
        e[2] * (c[0] - a[0]) - e[0] * (c[2] - a[2]),
        e[0] * (c[1] - a[1]) - e[1] * (c[0] - a[0]),
    ];
    let n2 = [
        e[1] * (d[2] - a[2]) - e[2] * (d[1] - a[1]),
        e[2] * (d[0] - a[0]) - e[0] * (d[2] - a[2]),
        e[0] * (d[1] - a[1]) - e[1] * (d[0] - a[0]),
    ];
    let len1 = (n1[0] * n1[0] + n1[1] * n1[1] + n1[2] * n1[2]).sqrt();
    let len2 = (n2[0] * n2[0] + n2[1] * n2[1] + n2[2] * n2[2]).sqrt();
    let n1n = [n1[0] / len1, n1[1] / len1, n1[2] / len1];
    let n2n = [n2[0] / len2, n2[1] / len2, n2[2] / len2];
    let cross = [
        n1n[1] * n2n[2] - n1n[2] * n2n[1],
        n1n[2] * n2n[0] - n1n[0] * n2n[2],
        n1n[0] * n2n[1] - n1n[1] * n2n[0],
    ];
    let s = (e[0] * cross[0] + e[1] * cross[1] + e[2] * cross[2]) / elen;
    let cos = n1n[0] * n2n[0] + n1n[1] * n2n[1] + n1n[2] * n2n[2];
    (-s).atan2(-cos)
}

#[test]
fn hinge_relaxes_toward_flat() {
    let dz = 0.2f32;
    let rest_positions = vec![
        0.0f32, 0.0, 0.0,
        0.0, 1.0, 0.0,
        1.0, 0.0, 0.0,
        -1.0, 0.0, 0.0,
    ];
    let positions = vec![
        0.0f32, 0.0, 0.0,
        0.0, 1.0, 0.0,
        1.0, 0.0, dz,
        -1.0, 0.0, dz,
    ];
    let triangles = vec![0u32, 1, 2, 0, 3, 1];
    let pairs = vec![0u32, 1, 0, 2, 1, 2, 0, 3, 1, 3];
    let rest = vec![1.0f32, 1.0, 2.0f32.sqrt(), 1.0, 2.0f32.sqrt()];
    let compliance = vec![1e-6f32; 5];
    let inverse_mass = vec![1.0f32; 4];
    let partner = vec![-1i32; 4];
    let contact_weight = vec![1.0f32; 4];
    let hinges = vec![0u32, 1, 2, 3];
    let hinge_rest = vec![dihedral(&rest_positions)];
    let hinge_compliance = vec![1e-3f32];
    let mut params = SolverParams::default();
    params.gravity = [0.0, 0.0, 0.0];
    params.frame_time = 1.0 / 30.0;
    params.seam_seconds = 0.8;
    params.damping = 1.0;
    params.self_thickness = 0.0;
    let mut solver = Solver::new(
        &positions,
        &inverse_mass,
        &pairs,
        &rest,
        &rest,
        &compliance,
        &triangles,
        &rest_positions,
        &partner,
        &contact_weight,
        &params,
        &hinges,
        &hinge_rest,
        &hinge_compliance,
    )
    .unwrap();
    let mut mask = vec![0u8; 4];
    mask[0] = 1;
    mask[1] = 1;
    let targets = positions.clone();
    solver.set_pins(&mask, &targets);
    let initial = dihedral(&positions);
    assert!(initial > 0.05, "expected a bent start, got {initial}");
    let mut previous = initial;
    for _ in 0..60 {
        solver.step(4, 2);
        let current = dihedral(&solver.positions());
        assert!(current.abs() <= initial, "dihedral exceeded initial ({} > {})", current.abs(), initial);
        previous = current;
    }
    assert!(previous.abs() < 0.02, "hinge did not relax, dihedral {previous}");
}
