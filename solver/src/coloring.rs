pub struct Coloring {
    pub n_colors: u32,
    pub order: Vec<u32>,
    pub offsets: Vec<u32>,
    pub counts: Vec<u32>,
}

fn finish_coloring(colors: Vec<u32>, n_colors: u32, count: usize) -> Coloring {
    let mut counts = vec![0u32; n_colors as usize];
    for &c in &colors {
        counts[c as usize] += 1;
    }
    let mut offsets = vec![0u32; n_colors as usize];
    let mut acc = 0u32;
    for c in 0..n_colors as usize {
        offsets[c] = acc;
        acc += counts[c];
    }
    let mut cursor = offsets.clone();
    let mut order = vec![0u32; count];
    for (old, &c) in colors.iter().enumerate() {
        let pos = cursor[c as usize] as usize;
        order[pos] = old as u32;
        cursor[c as usize] += 1;
    }
    Coloring {
        n_colors,
        order,
        offsets,
        counts,
    }
}

pub fn greedy(pairs: &[u32], n_vertices: usize) -> Result<Coloring, String> {
    let m = pairs.len() / 2;
    let mut colors = vec![0u32; m];
    let mut mask = vec![0u64; n_vertices];
    let mut max_color = 0u32;
    for (j, pair) in pairs.chunks_exact(2).enumerate() {
        let a = pair[0] as usize;
        let b = pair[1] as usize;
        let used = mask[a] | mask[b];
        let bit = (!used).trailing_zeros();
        if bit >= 64 {
            return Err("constraint graph needs more than 64 colours".to_string());
        }
        colors[j] = bit;
        mask[a] |= 1u64 << bit;
        mask[b] |= 1u64 << bit;
        max_color = max_color.max(bit);
    }
    Ok(finish_coloring(colors, max_color + 1, m))
}

pub fn greedy_hinges(hinges: &[u32], n_vertices: usize) -> Result<Coloring, String> {
    let k = hinges.len() / 4;
    let mut colors = vec![0u32; k];
    let mut mask = vec![0u64; n_vertices];
    let mut max_color = 0u32;
    for (j, quad) in hinges.chunks_exact(4).enumerate() {
        let used = mask[quad[0] as usize]
            | mask[quad[1] as usize]
            | mask[quad[2] as usize]
            | mask[quad[3] as usize];
        let bit = (!used).trailing_zeros();
        if bit >= 64 {
            return Err("hinge graph needs more than 64 colours".to_string());
        }
        colors[j] = bit;
        mask[quad[0] as usize] |= 1u64 << bit;
        mask[quad[1] as usize] |= 1u64 << bit;
        mask[quad[2] as usize] |= 1u64 << bit;
        mask[quad[3] as usize] |= 1u64 << bit;
        max_color = max_color.max(bit);
    }
    Ok(finish_coloring(colors, max_color + 1, k))
}
