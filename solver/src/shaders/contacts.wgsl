struct Params {
  h: f32,
  damping: f32,
  max_step: f32,
  gravity_ramp: f32,
  ramp: f32,
  body_margin: f32,
  body_grip: f32,
  body_step: f32,
  body_band: f32,
  self_thickness: f32,
  weld_reach: f32,
  weld_gain: f32,
  h2_inv: f32,
  n: u32,
  m: u32,
  t: u32,
  cells: u32,
  n_collider_tris: u32,
  hash_mask: u32,
  hash_s: u32,
  tris_offset: u32,
  cell_start_offset: u32,
  cell_items_offset: u32,
  collider_tris_offset: u32,
  gravity: vec4<f32>,
  grid_min: vec4<f32>,
  hash_min: vec4<f32>,
  grid_dims: vec4<u32>,
  grid_cell_size: f32,
  hash_cell_size: f32,
  inv_cell_size: f32,
  inv_hash_cell_size: f32,
  k: u32,
  pad0: f32,
  pad1: f32,
  pad2: f32,
}

struct VertAttr {
  partner: i32,
  weight: f32,
  pinned: u32,
}

struct ContactOut {
  delta: vec3<f32>,
  normal: vec3<f32>,
  touching: u32,
}

@group(0) @binding(0) var<storage, read> state: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> rest_pos: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> vert_attr: array<VertAttr>;
@group(0) @binding(3) var<storage, read_write> contact_out: array<ContactOut>;
@group(0) @binding(4) var<storage, read> hash_head: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read> hash_next: array<u32>;
@group(0) @binding(6) var<storage, read> collider_pos: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read> index_data: array<u32>;
@group(0) @binding(8) var<uniform> params: Params;

fn hash_cell(c: vec3<i32>) -> u32 {
  var h = 2166136261u;
  h = (h ^ u32(c.x)) * 16777619u;
  h = (h ^ u32(c.y)) * 16777619u;
  h = (h ^ u32(c.z)) * 16777619u;
  return h & params.hash_mask;
}

fn closest_bary(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
  let ab = b - a;
  let ac = c - a;
  let ap = p - a;
  let d1 = dot(ab, ap);
  let d2 = dot(ac, ap);
  if (d1 <= 0.0 && d2 <= 0.0) {
    return vec3<f32>(0.0, 0.0, 1.0);
  }
  let bp = p - b;
  let d3 = dot(ab, bp);
  let d4 = dot(ac, bp);
  if (d3 >= 0.0 && d4 <= d3) {
    return vec3<f32>(1.0, 0.0, 0.0);
  }
  let vc = d1 * d4 - d3 * d2;
  if (vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0) {
    let v = d1 / (d1 - d3);
    return vec3<f32>(v, 0.0, 1.0 - v);
  }
  let cp = p - c;
  let d5 = dot(ab, cp);
  let d6 = dot(ac, cp);
  if (d6 >= 0.0 && d5 <= d6) {
    return vec3<f32>(0.0, 1.0, 0.0);
  }
  let vb = d5 * d2 - d1 * d6;
  if (vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0) {
    let w = d2 / (d2 - d6);
    return vec3<f32>(0.0, w, 1.0 - w);
  }
  let va = d3 * d6 - d5 * d4;
  if (va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0) {
    let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
    return vec3<f32>(1.0 - w, w, 0.0);
  }
  let denom = 1.0 / (va + vb + vc);
  let v = vb * denom;
  let w = vc * denom;
  return vec3<f32>(v, w, 1.0 - v - w);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= params.n) { return; }
  let p = state[i];
  if (p.w == 0.0) { return; }
  if (vert_attr[i].pinned != 0u) { return; }
  var delta = vec3<f32>(0.0);
  var normal = vec3<f32>(0.0);
  var touching = 0u;

  let gp = p.xyz;
  let gc = floor((gp - params.grid_min.xyz) * params.inv_cell_size);
  let gci = vec3<i32>(gc);
  let dims = params.grid_dims.xyz;
  if (gci.x >= 0 && gci.y >= 0 && gci.z >= 0 && gci.x < i32(dims.x) && gci.y < i32(dims.y) && gci.z < i32(dims.z)) {
    let cell = u32(gci.x) + u32(gci.y) * dims.x + u32(gci.z) * dims.x * dims.y;
    let start = index_data[params.cell_start_offset + cell];
    let end = index_data[params.cell_start_offset + cell + 1u];
    var best = params.body_band;
    var found = false;
    var best_q = vec3<f32>(0.0);
    var best_n = vec3<f32>(0.0);
    for (var j = start; j < end; j = j + 1u) {
      let ti = index_data[params.cell_items_offset + j];
      let a = collider_pos[index_data[params.collider_tris_offset + ti * 3u]].xyz;
      let b = collider_pos[index_data[params.collider_tris_offset + ti * 3u + 1u]].xyz;
      let c = collider_pos[index_data[params.collider_tris_offset + ti * 3u + 2u]].xyz;
      let nraw = cross(b - a, c - a);
      let nl = length(nraw);
      if (nl < 1e-20) { continue; }
      let bary = closest_bary(gp, a, b, c);
      let q = a + (b - a) * bary.x + (c - a) * bary.y;
      let r = length(gp - q);
      if (r <= best) {
        best = r;
        found = true;
        best_q = q;
        best_n = nraw / nl;
      }
    }
    if (found) {
      let height = dot(gp - best_q, best_n);
      if (height < params.body_margin && (abs(height) > 0.6 * best || best < params.body_margin)) {
        touching = 1u;
        normal = best_n;
        delta = delta + best_n * min(params.body_margin - height, params.body_step);
      }
    }
  }

  if (params.self_thickness > 0.0) {
    var self_delta = vec3<f32>(0.0);
    let rc = floor((gp - params.hash_min.xyz) * params.inv_hash_cell_size);
    let rci = vec3<i32>(rc);
    let partner = vert_attr[i].partner;
    for (var dx = -1; dx <= 1; dx = dx + 1) {
      for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dz = -1; dz <= 1; dz = dz + 1) {
          let cell = rci + vec3<i32>(dx, dy, dz);
          let slot = hash_cell(cell);
          var tri = atomicLoad(&hash_head[slot]);
          while (tri != 0xFFFFFFFFu) {
            let a = index_data[params.tris_offset + tri * 3u];
            let b = index_data[params.tris_offset + tri * 3u + 1u];
            let c = index_data[params.tris_offset + tri * 3u + 2u];
            let shares_vertex = (a == i || b == i || c == i);
            let shares_partner = (partner >= 0 && (a == u32(partner) || b == u32(partner) || c == u32(partner)));
            if (!shares_vertex && !shares_partner) {
              let bary = closest_bary(gp, state[a].xyz, state[b].xyz, state[c].xyz);
              let q = state[a].xyz + (state[b].xyz - state[a].xyz) * bary.x + (state[c].xyz - state[a].xyz) * bary.y;
              let wq = vert_attr[a].weight * bary.z + vert_attr[b].weight * bary.x + vert_attr[c].weight * bary.y;
              let rbary = closest_bary(rest_pos[i].xyz, rest_pos[a].xyz, rest_pos[b].xyz, rest_pos[c].xyz);
              let rest_q = rest_pos[a].xyz + (rest_pos[b].xyz - rest_pos[a].xyz) * rbary.x + (rest_pos[c].xyz - rest_pos[a].xyz) * rbary.y;
              if (wq > 0.5 && vert_attr[i].weight > 0.5 && length(rest_q - rest_pos[i].xyz) > 4.0 * params.self_thickness) {
                let d = length(gp - q);
                if (d > 1e-9 && d < params.self_thickness) {
                  self_delta = self_delta + (gp - q) / d * (params.self_thickness - d) * 0.5;
                }
              }
            }
            tri = hash_next[tri];
          }
        }
      }
    }
    let sd = length(self_delta);
    if (sd > params.self_thickness) {
      self_delta = self_delta * (params.self_thickness / sd);
    }
    delta = delta + self_delta;
  }

  if (vert_attr[i].partner >= 0) {
    let xp = state[vert_attr[i].partner].xyz;
    if (length(xp - gp) < params.weld_reach) {
      delta = delta + (xp - gp) * params.weld_gain;
    }
  }

  contact_out[i] = ContactOut(delta, normal, touching);
}
