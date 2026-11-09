pub struct Grid {
    pub cell_size: f32,
    pub min: [f32; 3],
    pub dims: [u32; 3],
    pub cell_start: Vec<u32>,
    pub cell_items: Vec<u32>,
    pub cell_count: usize,
}

pub fn build(positions: &[f32], triangles: &[u32], body_band: f32) -> Grid {
    let cell_count = |min: [f32; 3], max: [f32; 3], cell_size: f32| -> ([u32; 3], usize) {
        let mut dims = [1u32; 3];
        let mut count = 1usize;
        for k in 0..3 {
            let span = (max[k] - min[k]).max(0.0);
            let n = if span <= 0.0 {
                1
            } else {
                (span / cell_size).ceil().max(1.0) as u32
            };
            dims[k] = n;
            count *= n as usize;
        }
        (dims, count)
    };

    if positions.is_empty() || triangles.is_empty() {
        return Grid {
            cell_size: body_band.max(1e-6),
            min: [0.0; 3],
            dims: [1, 1, 1],
            cell_start: vec![0, 0],
            cell_items: Vec::new(),
            cell_count: 1,
        };
    }

    let mut lo = [positions[0], positions[1], positions[2]];
    let mut hi = lo;
    for p in positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let min = [lo[0] - body_band, lo[1] - body_band, lo[2] - body_band];
    let max = [hi[0] + body_band, hi[1] + body_band, hi[2] + body_band];

    let mut cell_size = body_band.max(1e-6);
    let (dims, cells) = loop {
        let (dims, cells) = cell_count(min, max, cell_size);
        if cells <= 4_000_000 {
            break (dims, cells);
        }
        cell_size *= 1.5;
    };

    let mut counts = vec![0u32; cells];
    for tri in triangles.chunks_exact(3) {
        let a = [positions[tri[0] as usize * 3], positions[tri[0] as usize * 3 + 1], positions[tri[0] as usize * 3 + 2]];
        let b = [positions[tri[1] as usize * 3], positions[tri[1] as usize * 3 + 1], positions[tri[1] as usize * 3 + 2]];
        let c = [positions[tri[2] as usize * 3], positions[tri[2] as usize * 3 + 1], positions[tri[2] as usize * 3 + 2]];
        let mut tlo = a;
        let mut thi = a;
        for p in [b, c] {
            for k in 0..3 {
                tlo[k] = tlo[k].min(p[k]);
                thi[k] = thi[k].max(p[k]);
            }
        }
        for k in 0..3 {
            tlo[k] -= body_band;
            thi[k] += body_band;
        }
        let c0 = cell_index(min, cell_size, dims, tlo);
        let c1 = cell_index(min, cell_size, dims, thi);
        for cx in c0[0]..=c1[0] {
            for cy in c0[1]..=c1[1] {
                for cz in c0[2]..=c1[2] {
                    let idx = (cx as usize) + (cy as usize) * dims[0] as usize + (cz as usize) * dims[0] as usize * dims[1] as usize;
                    counts[idx] += 1;
                }
            }
        }
    }

    let mut cell_start = vec![0u32; cells + 1];
    for i in 0..cells {
        cell_start[i + 1] = cell_start[i] + counts[i];
    }
    let mut cursor = cell_start[..cells].to_vec();
    let mut cell_items = vec![0u32; cell_start[cells] as usize];
    for (tri_index, tri) in triangles.chunks_exact(3).enumerate() {
        let a = [positions[tri[0] as usize * 3], positions[tri[0] as usize * 3 + 1], positions[tri[0] as usize * 3 + 2]];
        let b = [positions[tri[1] as usize * 3], positions[tri[1] as usize * 3 + 1], positions[tri[1] as usize * 3 + 2]];
        let c = [positions[tri[2] as usize * 3], positions[tri[2] as usize * 3 + 1], positions[tri[2] as usize * 3 + 2]];
        let mut tlo = a;
        let mut thi = a;
        for p in [b, c] {
            for k in 0..3 {
                tlo[k] = tlo[k].min(p[k]);
                thi[k] = thi[k].max(p[k]);
            }
        }
        for k in 0..3 {
            tlo[k] -= body_band;
            thi[k] += body_band;
        }
        let c0 = cell_index(min, cell_size, dims, tlo);
        let c1 = cell_index(min, cell_size, dims, thi);
        for cx in c0[0]..=c1[0] {
            for cy in c0[1]..=c1[1] {
                for cz in c0[2]..=c1[2] {
                    let idx = (cx as usize) + (cy as usize) * dims[0] as usize + (cz as usize) * dims[0] as usize * dims[1] as usize;
                    cell_items[cursor[idx] as usize] = tri_index as u32;
                    cursor[idx] += 1;
                }
            }
        }
    }

    Grid {
        cell_size,
        min,
        dims,
        cell_start,
        cell_items,
        cell_count: cells,
    }
}

fn cell_index(min: [f32; 3], cell_size: f32, dims: [u32; 3], p: [f32; 3]) -> [u32; 3] {
    let mut out = [0u32; 3];
    for k in 0..3 {
        let f = ((p[k] - min[k]) / cell_size).floor();
        let c = if f < 0.0 {
            0
        } else if f >= dims[k] as f32 {
            dims[k] - 1
        } else {
            f as u32
        };
        out[k] = c;
    }
    out
}
