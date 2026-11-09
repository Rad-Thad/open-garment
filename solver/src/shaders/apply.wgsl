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
  pad: vec4<f32>,
}

struct ContactOut {
  delta: vec3<f32>,
  normal: vec3<f32>,
  touching: u32,
}

@group(0) @binding(0) var<storage, read_write> state: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> contact_out: array<ContactOut>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= params.n) { return; }
  let p = state[i];
  if (p.w == 0.0) { return; }
  state[i] = vec4<f32>(p.xyz + contact_out[i].delta, p.w);
}
