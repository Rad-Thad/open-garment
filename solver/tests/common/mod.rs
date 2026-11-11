#![allow(dead_code)]

use garment_solver::engine::Solver;

pub fn run_frames(solver: &mut Solver, frames: usize, substeps: u32, iterations: u32) {
    let frame_time = solver.params().frame_time;
    for f in 0..frames {
        let mut p = *solver.params();
        p.elapsed = f as f32 * frame_time;
        solver.set_params(p);
        solver.step(substeps, iterations).unwrap();
    }
}

pub struct Cloth {
    pub positions: Vec<f32>,
    pub inverse_mass: Vec<f32>,
    pub pairs: Vec<u32>,
    pub rest: Vec<f32>,
    pub rest_start: Vec<f32>,
    pub compliance: Vec<f32>,
    pub triangles: Vec<u32>,
    pub rest_positions: Vec<f32>,
    pub partner: Vec<i32>,
    pub contact_weight: Vec<f32>,
}

pub fn grid_cloth(nx: usize, ny: usize, spacing: f32, compliance: f32) -> Cloth {
    let count = nx * ny;
    let mut positions = Vec::with_capacity(count * 3);
    for i in 0..ny {
        for j in 0..nx {
            positions.extend_from_slice(&[j as f32 * spacing, i as f32 * spacing, 0.0]);
        }
    }
    let index = |i: usize, j: usize| i * nx + j;
    let mut pairs = Vec::new();
    let mut rest = Vec::new();
    for i in 0..ny {
        for j in 0..nx {
            if j + 1 < nx {
                pairs.extend_from_slice(&[index(i, j) as u32, index(i, j + 1) as u32]);
                rest.push(spacing);
            }
            if i + 1 < ny {
                pairs.extend_from_slice(&[index(i, j) as u32, index(i + 1, j) as u32]);
                rest.push(spacing);
            }
            if i + 1 < ny && j + 1 < nx {
                pairs.extend_from_slice(&[index(i, j) as u32, index(i + 1, j + 1) as u32]);
                rest.push(spacing * std::f32::consts::SQRT_2);
                pairs.extend_from_slice(&[index(i, j + 1) as u32, index(i + 1, j) as u32]);
                rest.push(spacing * std::f32::consts::SQRT_2);
            }
        }
    }
    let mut triangles = Vec::new();
    for i in 0..ny - 1 {
        for j in 0..nx - 1 {
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
    Cloth {
        positions,
        inverse_mass: vec![1.0; count],
        pairs,
        rest_start: rest.clone(),
        compliance: vec![compliance; m],
        rest,
        triangles,
        rest_positions: {
            let mut r = Vec::with_capacity(count * 3);
            for i in 0..ny {
                for j in 0..nx {
                    r.extend_from_slice(&[j as f32 * spacing, i as f32 * spacing, 0.0]);
                }
            }
            r
        },
        partner: vec![-1; count],
        contact_weight: vec![1.0; count],
    }
}

pub fn merge(a: Cloth, b: Cloth) -> Cloth {
    let offset = (a.positions.len() / 3) as u32;
    let mut pairs = a.pairs;
    pairs.extend(b.pairs.iter().map(|v| v + offset));
    let mut triangles = a.triangles;
    triangles.extend(b.triangles.iter().map(|v| v + offset));
    let mut positions = a.positions;
    positions.extend(b.positions);
    let mut inverse_mass = a.inverse_mass;
    inverse_mass.extend(b.inverse_mass);
    let mut rest = a.rest;
    rest.extend(b.rest);
    let mut rest_start = a.rest_start;
    rest_start.extend(b.rest_start);
    let mut compliance = a.compliance;
    compliance.extend(b.compliance);
    let mut rest_positions = a.rest_positions;
    rest_positions.extend(b.rest_positions);
    let mut partner = a.partner;
    partner.extend(b.partner.iter().map(|&p| if p >= 0 { p + offset as i32 } else { p }));
    let mut contact_weight = a.contact_weight;
    contact_weight.extend(b.contact_weight);
    Cloth {
        positions,
        inverse_mass,
        pairs,
        rest,
        rest_start,
        compliance,
        triangles,
        rest_positions,
        partner,
        contact_weight,
    }
}

pub fn uv_sphere(radius: f32, segments: usize, rings: usize) -> (Vec<f32>, Vec<u32>) {
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
            triangles.extend_from_slice(&[a, c, b]);
            triangles.extend_from_slice(&[b, c, d]);
        }
    }
    (vertices, triangles)
}

pub fn box_collider(half: [f32; 3]) -> (Vec<f32>, Vec<u32>) {
    let h = half;
    let corners = [
        [-h[0], -h[1], -h[2]],
        [h[0], -h[1], -h[2]],
        [h[0], h[1], -h[2]],
        [-h[0], h[1], -h[2]],
        [-h[0], -h[1], h[2]],
        [h[0], -h[1], h[2]],
        [h[0], h[1], h[2]],
        [-h[0], h[1], h[2]],
    ];
    let mut vertices = Vec::new();
    for c in corners {
        vertices.extend_from_slice(&c);
    }
    let faces = [
        [0, 1, 2],
        [0, 2, 3],
        [4, 6, 5],
        [4, 7, 6],
        [0, 4, 5],
        [0, 5, 1],
        [1, 5, 6],
        [1, 6, 2],
        [2, 6, 7],
        [2, 7, 3],
        [3, 7, 4],
        [3, 4, 0],
    ];
    let mut triangles = Vec::new();
    for f in faces {
        triangles.extend_from_slice(&[f[0] as u32, f[1] as u32, f[2] as u32]);
    }
    (vertices, triangles)
}

pub fn closest_bary(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let dot = |u: [f32; 3], v: [f32; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return [0.0, 0.0, 1.0];
    }
    let bp = [p[0] - b[0], p[1] - b[1], p[2] - b[2]];
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return [1.0, 0.0, 0.0];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return [v, 0.0, 1.0 - v];
    }
    let cp = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return [0.0, 1.0, 0.0];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return [0.0, w, 1.0 - w];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return [1.0 - w, w, 0.0];
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    [v, w, 1.0 - v - w]
}

pub fn tri_point(bary: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * bary[0] + (c[0] - a[0]) * bary[1],
        a[1] + (b[1] - a[1]) * bary[0] + (c[1] - a[1]) * bary[1],
        a[2] + (b[2] - a[2]) * bary[0] + (c[2] - a[2]) * bary[1],
    ]
}

pub fn dist_point_tri(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let q = tri_point(closest_bary(p, a, b, c), a, b, c);
    let dx = p[0] - q[0];
    let dy = p[1] - q[1];
    let dz = p[2] - q[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub fn vertex(p: &[f32], i: usize) -> [f32; 3] {
    [p[3 * i], p[3 * i + 1], p[3 * i + 2]]
}
