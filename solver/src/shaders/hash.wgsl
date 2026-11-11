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

@group(0) @binding(0) var<storage, read_write> hash_head: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> hash_next: array<u32>;
@group(0) @binding(2) var<storage, read> state: array<vec4<f32>>;
@group(0) @binding(3) var<storage, read> index_data: array<u32>;
@group(0) @binding(4) var<uniform> params: Params;

fn hash_cell(c: vec3<i32>) -> u32 {
  var h = 2166136261u;
  h = (h ^ u32(c.x)) * 16777619u;
  h = (h ^ u32(c.y)) * 16777619u;
  h = (h ^ u32(c.z)) * 16777619u;
  return h & params.hash_mask;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let tri = gid.x;
  if (tri >= params.t) { return; }
  let a = index_data[params.tris_offset + tri * 3u];
  let b = index_data[params.tris_offset + tri * 3u + 1u];
  let c = index_data[params.tris_offset + tri * 3u + 2u];
  let centroid = (state[a].xyz + state[b].xyz + state[c].xyz) / 3.0;
  let cell = vec3<i32>(floor((centroid - params.hash_min.xyz) * params.inv_hash_cell_size));
  let slot = hash_cell(cell);
  let old = atomicExchange(&hash_head[slot], tri);
  hash_next[tri] = old;
}
