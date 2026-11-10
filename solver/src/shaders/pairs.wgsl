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

struct ColorSlot {
  offset: u32,
  count: u32,
}

@group(0) @binding(0) var<storage, read_write> state: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> constraints: array<vec4<f32>>;
@group(0) @binding(2) var<storage, read> pairs: array<u32>;
@group(0) @binding(3) var<storage, read> vert_attr: array<VertAttr>;
@group(0) @binding(4) var<uniform> params: Params;
@group(0) @binding(5) var<uniform> slot: ColorSlot;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (gid.x >= slot.count) { return; }
  let j = slot.offset + gid.x;
  let a = pairs[j * 2u];
  let b = pairs[j * 2u + 1u];
  var pa = state[a];
  var pb = state[b];
  let wa = select(pa.w, 0.0, vert_attr[a].pinned != 0u);
  let wb = select(pb.w, 0.0, vert_attr[b].pinned != 0u);
  let wsum = wa + wb;
  if (wsum == 0.0) { return; }
  let diff = pa.xyz - pb.xyz;
  let dist = length(diff);
  if (dist < 1e-9) { return; }
  let c = constraints[j];
  let rest_t = mix(c.x, c.y, params.ramp);
  let alpha = c.z * params.h2_inv;
  let dl = (-(dist - rest_t) - alpha * c.w) / (wsum + alpha);
  constraints[j] = vec4<f32>(c.x, c.y, c.z, c.w + dl);
  let corr = (diff / dist) * dl;
  state[a] = vec4<f32>(pa.xyz + corr * wa, pa.w);
  state[b] = vec4<f32>(pb.xyz - corr * wb, pb.w);
}
