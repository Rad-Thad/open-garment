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

@group(0) @binding(0) var<storage, read_write> constraints: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> contact_out: array<ContactOut>;
@group(0) @binding(2) var<storage, read_write> hash_head: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> hash_next: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i < params.m) {
    let c = constraints[i];
    constraints[i] = vec4<f32>(c.x, c.y, c.z, 0.0);
  }
  if (i < params.n) {
    contact_out[i] = ContactOut(vec3<f32>(0.0), vec3<f32>(0.0), 0u);
  }
  if (i < params.hash_s) {
    atomicStore(&hash_head[i], 0xFFFFFFFFu);
  }
  if (i < params.t) {
    hash_next[i] = 0xFFFFFFFFu;
  }
}
