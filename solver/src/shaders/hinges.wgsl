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

struct HingeSlot {
  offset: u32,
  count: u32,
}

@group(0) @binding(0) var<storage, read_write> state: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> vert_attr: array<VertAttr>;
@group(0) @binding(2) var<storage, read> hinges: array<u32>;
@group(0) @binding(3) var<storage, read> hinge_rest: array<f32>;
@group(0) @binding(4) var<storage, read> hinge_compliance: array<f32>;
@group(0) @binding(5) var<storage, read_write> hinge_lambda: array<f32>;
@group(0) @binding(6) var<uniform> params: Params;
@group(0) @binding(7) var<uniform> slot: HingeSlot;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= slot.count) { return; }
  let j = slot.offset + gid.x;
  let a = hinges[j * 4u];
  let b = hinges[j * 4u + 1u];
  let c = hinges[j * 4u + 2u];
  let d = hinges[j * 4u + 3u];
  let x1 = state[a].xyz;
  let x2 = state[b].xyz;
  let x3 = state[c].xyz;
  let x4 = state[d].xyz;
  let e = x2 - x1;
  let elen = length(e);
  if (elen < 1e-9) { return; }
  let n1 = cross(e, x3 - x1);
  let n2 = cross(e, x4 - x1);
  let n1len = length(n1);
  let n2len = length(n2);
  if (n1len < 1e-12 || n2len < 1e-12) { return; }
  let n1n = n1 / n1len;
  let n2n = n2 / n2len;
  let s = dot(e, cross(n1n, n2n)) / elen;
  let cos_theta = dot(n1n, n2n);
  let theta = atan2(s, cos_theta);
  let rest = hinge_rest[j];
  let error = theta - rest;
  let g3 = (elen / (n1len * n1len)) * n1;
  let g4 = -(elen / (n2len * n2len)) * n2;
  let g2 = (dot(x1 - x3, e) / (elen * elen)) * g3 + (dot(x1 - x4, e) / (elen * elen)) * g4;
  let g1 = -g2 - g3 - g4;
  let wa = select(state[a].w, 0.0, vert_attr[a].pinned != 0u);
  let wb = select(state[b].w, 0.0, vert_attr[b].pinned != 0u);
  let wc = select(state[c].w, 0.0, vert_attr[c].pinned != 0u);
  let wd = select(state[d].w, 0.0, vert_attr[d].pinned != 0u);
  let wsum = wa * dot(g1, g1) + wb * dot(g2, g2) + wc * dot(g3, g3) + wd * dot(g4, g4);
  let alpha = hinge_compliance[j] * params.h2_inv;
  let dl = -(error + alpha * hinge_lambda[j]) / (wsum + alpha);
  hinge_lambda[j] = hinge_lambda[j] + dl;
  state[a] = vec4<f32>(x1 + wa * dl * g1, state[a].w);
  state[b] = vec4<f32>(x2 + wb * dl * g2, state[b].w);
  state[c] = vec4<f32>(x3 + wc * dl * g3, state[c].w);
  state[d] = vec4<f32>(x4 + wd * dl * g4, state[d].w);
}
