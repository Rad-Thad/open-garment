use bytemuck::{Pod, Zeroable};
use pollster::block_on;
use wgpu::util::DeviceExt;

use crate::coloring::{self, greedy_hinges};
use crate::grid::{build as build_grid, Grid};

const WORKGROUP: u32 = 64;
const SLOT: u32 = 256;
const MAX_SUBSTEPS: u32 = 64;
const MAX_COLORS: u32 = 64;
const CONTACT_STRIDE: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
#[allow(dead_code)]
pub struct Params {
    pub h: f32,
    pub damping: f32,
    pub max_step: f32,
    pub gravity_ramp: f32,
    pub ramp: f32,
    pub body_margin: f32,
    pub body_grip: f32,
    pub body_step: f32,
    pub body_band: f32,
    pub self_thickness: f32,
    pub weld_reach: f32,
    pub weld_gain: f32,
    pub h2_inv: f32,
    pub n: u32,
    pub m: u32,
    pub t: u32,
    pub cells: u32,
    pub n_collider_tris: u32,
    pub hash_mask: u32,
    pub hash_s: u32,
    pub tris_offset: u32,
    pub cell_start_offset: u32,
    pub cell_items_offset: u32,
    pub collider_tris_offset: u32,
    pub gravity: [f32; 4],
    pub grid_min: [f32; 4],
    pub hash_min: [f32; 4],
    pub grid_dims: [u32; 4],
    pub grid_cell_size: f32,
    pub hash_cell_size: f32,
    pub inv_cell_size: f32,
    pub inv_hash_cell_size: f32,
    pub k: u32,
    pub pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VertAttr {
    partner: i32,
    weight: f32,
    pinned: u32,
}

#[derive(Clone, Copy)]
pub struct SolverParams {
    pub frame_time: f32,
    pub elapsed: f32,
    pub seam_seconds: f32,
    pub gravity: [f32; 3],
    pub damping: f32,
    pub body_margin: f32,
    pub body_grip: f32,
    pub body_step: f32,
    pub body_band: f32,
    pub self_thickness: f32,
    pub weld_reach: f32,
    pub weld_gain: f32,
    pub max_step: f32,
}

impl Default for SolverParams {
    fn default() -> Self {
        SolverParams {
            frame_time: 1.0 / 30.0,
            elapsed: 0.0,
            seam_seconds: 0.8,
            gravity: [0.0, 0.0, -9.81],
            damping: 1.0,
            body_margin: 0.003,
            body_grip: 0.1,
            body_step: 0.01,
            body_band: 0.03,
            self_thickness: 0.0,
            weld_reach: 0.02,
            weld_gain: 0.25,
            max_step: 0.015,
        }
    }
}

struct Pipelines {
    integrate: wgpu::ComputePipeline,
    clear: wgpu::ComputePipeline,
    pairs: wgpu::ComputePipeline,
    hinges: wgpu::ComputePipeline,
    hash: wgpu::ComputePipeline,
    contacts: wgpu::ComputePipeline,
    apply: wgpu::ComputePipeline,
    finish: wgpu::ComputePipeline,
}

struct Layouts {
    integrate: wgpu::BindGroupLayout,
    clear: wgpu::BindGroupLayout,
    pairs: wgpu::BindGroupLayout,
    hinges: wgpu::BindGroupLayout,
    hash: wgpu::BindGroupLayout,
    contacts: wgpu::BindGroupLayout,
    apply: wgpu::BindGroupLayout,
    finish: wgpu::BindGroupLayout,
}

struct Bg {
    integrate: wgpu::BindGroup,
    clear: wgpu::BindGroup,
    pairs: wgpu::BindGroup,
    hinges: wgpu::BindGroup,
    hash: wgpu::BindGroup,
    contacts: wgpu::BindGroup,
    apply: wgpu::BindGroup,
    finish: wgpu::BindGroup,
}

fn storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn storage_rw(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform(binding: u32, dynamic: bool, min_size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: dynamic,
            min_binding_size: if dynamic {
                Some(std::num::NonZeroU64::new(min_size).unwrap())
            } else {
                None
            },
        },
        count: None,
    }
}

fn create_layouts(device: &wgpu::Device) -> Layouts {
    let params_size = std::mem::size_of::<Params>() as u64;
    let color_size = 8u64;
    Layouts {
        integrate: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("integrate"),
            entries: &[storage_rw(0), storage_ro(1), storage_ro(2), uniform(3, true, params_size)],
        }),
        clear: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("clear"),
            entries: &[storage_rw(0), storage_rw(1), storage_rw(2), storage_rw(3), storage_rw(4), uniform(5, true, params_size)],
        }),
        pairs: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pairs"),
            entries: &[
                storage_rw(0),
                storage_rw(1),
                storage_ro(2),
                storage_ro(3),
                uniform(4, true, params_size),
                uniform(5, true, color_size),
            ],
        }),
        hinges: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hinges"),
            entries: &[
                storage_rw(0),
                storage_ro(1),
                storage_ro(2),
                storage_ro(3),
                storage_ro(4),
                storage_rw(5),
                uniform(6, true, params_size),
                uniform(7, true, color_size),
            ],
        }),
        hash: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hash"),
            entries: &[storage_rw(0), storage_rw(1), storage_ro(2), storage_ro(3), uniform(4, true, params_size)],
        }),
        contacts: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("contacts"),
            entries: &[
                storage_ro(0),
                storage_ro(1),
                storage_ro(2),
                storage_rw(3),
                storage_ro(4),
                storage_ro(5),
                storage_ro(6),
                storage_ro(7),
                uniform(8, true, params_size),
            ],
        }),
        apply: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("apply"),
            entries: &[storage_rw(0), storage_ro(1), uniform(2, true, params_size)],
        }),
        finish: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("finish"),
            entries: &[storage_rw(0), storage_ro(1), uniform(2, true, params_size)],
        }),
    }
}

fn create_pipelines(device: &wgpu::Device, layouts: &Layouts) -> Pipelines {
    let make = |label: &str, src: &str, layout: &wgpu::BindGroupLayout| {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };
    Pipelines {
        integrate: make("integrate", include_str!("shaders/integrate.wgsl"), &layouts.integrate),
        clear: make("clear", include_str!("shaders/clear.wgsl"), &layouts.clear),
        pairs: make("pairs", include_str!("shaders/pairs.wgsl"), &layouts.pairs),
        hinges: make("hinges", include_str!("shaders/hinges.wgsl"), &layouts.hinges),
        hash: make("hash", include_str!("shaders/hash.wgsl"), &layouts.hash),
        contacts: make("contacts", include_str!("shaders/contacts.wgsl"), &layouts.contacts),
        apply: make("apply", include_str!("shaders/apply.wgsl"), &layouts.apply),
        finish: make("finish", include_str!("shaders/finish.wgsl"), &layouts.finish),
    }
}

fn entry<'a>(binding: u32, buffer: &'a wgpu::Buffer) -> wgpu::BindGroupEntry<'a> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn sized_entry<'a>(binding: u32, buffer: &'a wgpu::Buffer, size: u64) -> wgpu::BindGroupEntry<'a> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset: 0,
            size: Some(std::num::NonZeroU64::new(size).unwrap()),
        }),
    }
}

fn build_bind_groups(
    device: &wgpu::Device,
    layouts: &Layouts,
    state: &wgpu::Buffer,
    pin: &wgpu::Buffer,
    vert_attr: &wgpu::Buffer,
    constraints: &wgpu::Buffer,
    pairs: &wgpu::Buffer,
    contact_out: &wgpu::Buffer,
    hash_head: &wgpu::Buffer,
    hash_next: &wgpu::Buffer,
    rest_pos: &wgpu::Buffer,
    collider_pos: &wgpu::Buffer,
    index_data: &wgpu::Buffer,
    params: &wgpu::Buffer,
    color: &wgpu::Buffer,
    hinges: &wgpu::Buffer,
    hinge_rest: &wgpu::Buffer,
    hinge_compliance: &wgpu::Buffer,
    hinge_lambda: &wgpu::Buffer,
    hinge_color: &wgpu::Buffer,
) -> Bg {
    let params_size = std::mem::size_of::<Params>() as u64;
    let color_size = 8u64;
    let params_entry = |binding: u32| sized_entry(binding, params, params_size);
    let color_entry = |binding: u32| sized_entry(binding, color, color_size);
    Bg {
        integrate: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("integrate"),
            layout: &layouts.integrate,
            entries: &[entry(0, state), entry(1, pin), entry(2, vert_attr), params_entry(3)],
        }),
        clear: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("clear"),
            layout: &layouts.clear,
            entries: &[
                entry(0, constraints),
                entry(1, contact_out),
                entry(2, hash_head),
                entry(3, hash_next),
                entry(4, hinge_lambda),
                params_entry(5),
            ],
        }),
        pairs: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pairs"),
            layout: &layouts.pairs,
            entries: &[
                entry(0, state),
                entry(1, constraints),
                entry(2, pairs),
                entry(3, vert_attr),
                params_entry(4),
                color_entry(5),
            ],
        }),
        hinges: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hinges"),
            layout: &layouts.hinges,
            entries: &[
                entry(0, state),
                entry(1, vert_attr),
                entry(2, hinges),
                entry(3, hinge_rest),
                entry(4, hinge_compliance),
                entry(5, hinge_lambda),
                params_entry(6),
                sized_entry(7, hinge_color, 8),
            ],
        }),
        hash: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hash"),
            layout: &layouts.hash,
            entries: &[
                entry(0, hash_head),
                entry(1, hash_next),
                entry(2, state),
                entry(3, index_data),
                params_entry(4),
            ],
        }),
        contacts: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("contacts"),
            layout: &layouts.contacts,
            entries: &[
                entry(0, state),
                entry(1, rest_pos),
                entry(2, vert_attr),
                entry(3, contact_out),
                entry(4, hash_head),
                entry(5, hash_next),
                entry(6, collider_pos),
                entry(7, index_data),
                params_entry(8),
            ],
        }),
        apply: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apply"),
            layout: &layouts.apply,
            entries: &[entry(0, state), entry(1, contact_out), params_entry(2)],
        }),
        finish: device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("finish"),
            layout: &layouts.finish,
            entries: &[entry(0, state), entry(1, contact_out), params_entry(2)],
        }),
    }
}

fn storage_buffer(device: &wgpu::Device, data: &[u8], label: &str, copy_src: bool) -> wgpu::Buffer {
    let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    if copy_src {
        usage |= wgpu::BufferUsages::COPY_SRC;
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage,
    })
}

fn div_ceil(a: u32, b: u32) -> u32 {
    (a + b - 1) / b
}

fn next_pow2(v: usize) -> usize {
    if v <= 1 {
        return 1;
    }
    1usize << (usize::BITS - (v - 1).leading_zeros())
}

pub struct Solver {
    device: wgpu::Device,
    queue: wgpu::Queue,
    state: wgpu::Buffer,
    pin: wgpu::Buffer,
    vert_attr: wgpu::Buffer,
    constraints: wgpu::Buffer,
    pairs: wgpu::Buffer,
    contact_out: wgpu::Buffer,
    hash_head: wgpu::Buffer,
    hash_next: wgpu::Buffer,
    rest_pos: wgpu::Buffer,
    collider_pos: wgpu::Buffer,
    index_data: wgpu::Buffer,
    params_buf: wgpu::Buffer,
    color_buf: wgpu::Buffer,
    hinges: wgpu::Buffer,
    hinge_rest: wgpu::Buffer,
    hinge_compliance: wgpu::Buffer,
    hinge_lambda: wgpu::Buffer,
    hinge_color_buf: wgpu::Buffer,
    staging: wgpu::Buffer,
    pipelines: Pipelines,
    layouts: Layouts,
    bg: Bg,
    n: u32,
    m: u32,
    t: u32,
    k: u32,
    n_collider_tris: u32,
    n_colors: u32,
    color_counts: Vec<u32>,
    hinge_n_colors: u32,
    hinge_color_counts: Vec<u32>,
    hash_s: u32,
    hash_mask: u32,
    hash_min: [f32; 4],
    hash_cell_size: f32,
    inv_hash_cell_size: f32,
    grid: Grid,
    tris_offset: u32,
    cell_start_offset: u32,
    cell_items_offset: u32,
    collider_tris_offset: u32,
    params: SolverParams,
    vert_attr_host: Vec<VertAttr>,
    cloth_tris_host: Vec<u32>,
    cached: Vec<f32>,
}

impl Solver {
    pub fn adapter_name() -> Result<String, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| e.to_string())?;
        Ok(adapter.get_info().name)
    }

    pub fn new(
        positions: &[f32],
        inverse_mass: &[f32],
        pairs: &[u32],
        rest: &[f32],
        rest_start: &[f32],
        compliance: &[f32],
        triangles: &[u32],
        rest_positions: &[f32],
        partner: &[i32],
        contact_weight: &[f32],
        params: &SolverParams,
        hinges: &[u32],
        hinge_rest: &[f32],
        hinge_compliance: &[f32],
    ) -> Result<Self, String> {
        let n = positions.len() / 3;
        let m = pairs.len() / 2;
        let t = triangles.len() / 3;
        let k = hinges.len() / 4;
        if n == 0 {
            return Err("zero vertices".to_string());
        }
        if m == 0 {
            return Err("zero pairs".to_string());
        }
        if inverse_mass.len() != n
            || rest.len() != m
            || rest_start.len() != m
            || compliance.len() != m
            || rest_positions.len() != n * 3
            || partner.len() != n
            || contact_weight.len() != n
            || hinge_rest.len() != k
            || hinge_compliance.len() != k
        {
            return Err("array length mismatch".to_string());
        }

        let coloring = coloring::greedy(pairs, n)?;

        let mut reordered_pairs = vec![0u32; 2 * m];
        let mut reordered_constraints = vec![[0.0f32; 4]; m];
        for (new_pos, &old) in coloring.order.iter().enumerate() {
            let old = old as usize;
            reordered_pairs[2 * new_pos] = pairs[2 * old];
            reordered_pairs[2 * new_pos + 1] = pairs[2 * old + 1];
            reordered_constraints[new_pos] =
                [rest_start[old], rest[old], compliance[old], 0.0];
        }

        let mut color_bytes = vec![0u8; (MAX_COLORS * SLOT) as usize];
        for c in 0..coloring.n_colors as usize {
            let base = c * SLOT as usize;
            color_bytes[base..base + 4].copy_from_slice(&coloring.offsets[c].to_le_bytes());
            color_bytes[base + 4..base + 8].copy_from_slice(&coloring.counts[c].to_le_bytes());
        }

        let hinge_coloring = greedy_hinges(hinges, n)?;
        let mut reordered_hinges = vec![0u32; 4 * k];
        let mut reordered_hinge_rest = vec![0.0f32; k];
        let mut reordered_hinge_compliance = vec![0.0f32; k];
        for (new_pos, &old) in hinge_coloring.order.iter().enumerate() {
            let old = old as usize;
            for j in 0..4 {
                reordered_hinges[4 * new_pos + j] = hinges[4 * old + j];
            }
            reordered_hinge_rest[new_pos] = hinge_rest[old];
            reordered_hinge_compliance[new_pos] = hinge_compliance[old];
        }
        let mut hinge_color_bytes = vec![0u8; (MAX_COLORS * SLOT) as usize];
        for c in 0..hinge_coloring.n_colors as usize {
            let base = c * SLOT as usize;
            hinge_color_bytes[base..base + 4].copy_from_slice(&hinge_coloring.offsets[c].to_le_bytes());
            hinge_color_bytes[base + 4..base + 8].copy_from_slice(&hinge_coloring.counts[c].to_le_bytes());
        }

        let mut state_data = vec![[0.0f32; 4]; 2 * n];
        for i in 0..n {
            state_data[i] = [positions[3 * i], positions[3 * i + 1], positions[3 * i + 2], inverse_mass[i]];
            state_data[n + i] = [positions[3 * i], positions[3 * i + 1], positions[3 * i + 2], 0.0];
        }
        let mut rest_data = vec![[0.0f32; 4]; n];
        for i in 0..n {
            rest_data[i] = [rest_positions[3 * i], rest_positions[3 * i + 1], rest_positions[3 * i + 2], 0.0];
        }
        let vert_attr_host: Vec<VertAttr> = (0..n)
            .map(|i| VertAttr {
                partner: partner[i],
                weight: contact_weight[i],
                pinned: 0,
            })
            .collect();

        let r_max = max_tri_radius(triangles, rest_positions);
        let hash_cell_size = if params.self_thickness > 0.0 || r_max > 0.0 {
            params.self_thickness + 1.2 * r_max
        } else {
            1.0
        }
        .max(1e-6);
        let hash_s = next_pow2((2 * t).max(1)).max(1024);
        let hash_mask = (hash_s - 1) as u32;
        let mut hash_min = [0.0f32; 4];
        if n > 0 {
            let mut lo = [f32::INFINITY; 3];
            for p in rest_positions.chunks_exact(3) {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                }
            }
            for k in 0..3 {
                hash_min[k] = lo[k];
            }
        }
        let inv_hash_cell_size = 1.0 / hash_cell_size;

        let grid = build_grid(&[], &[], params.body_band);
        let (index_data, tris_offset, cell_start_offset, cell_items_offset, collider_tris_offset) =
            build_index_data(triangles, &grid, &[]);
        let n_collider_tris = 0u32;
        let collider_pos_data = vec![0.0f32; 4];

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| format!("No GPU adapter: {e}"))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("garment_solver"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            ..Default::default()
        }))
        .map_err(|e| format!("No GPU adapter: {e}"))?;

        let state = storage_buffer(&device, bytemuck::cast_slice(&state_data), "state", true);
        let pin = storage_buffer(&device, &vec![0u8; n * 16], "pin", false);
        let vert_attr = storage_buffer(&device, bytemuck::cast_slice(&vert_attr_host), "vert_attr", false);
        let constraints = storage_buffer(
            &device,
            bytemuck::cast_slice(&reordered_constraints),
            "constraints",
            false,
        );
        let pairs = storage_buffer(&device, bytemuck::cast_slice(&reordered_pairs), "pairs", false);
        let contact_out = storage_buffer(&device, &vec![0u8; n * CONTACT_STRIDE], "contact_out", false);
        let hash_head = storage_buffer(&device, &vec![0xFFu8; (hash_s * 4) as usize], "hash_head", false);
        let hash_next = storage_buffer(&device, &vec![0xFFu8; (t * 4).max(4)], "hash_next", false);
        let rest_pos = storage_buffer(&device, bytemuck::cast_slice(&rest_data), "rest_pos", false);
        let collider_pos = storage_buffer(&device, bytemuck::cast_slice(&collider_pos_data), "collider_pos", false);
        let index_data = storage_buffer(&device, bytemuck::cast_slice(&index_data), "index_data", false);
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params"),
            size: (MAX_SUBSTEPS * SLOT) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let color_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("color"),
            size: (MAX_COLORS * SLOT) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&color_buf, 0, &color_bytes);
        let hinge_hinges_data = if k == 0 { vec![0u32; 4] } else { reordered_hinges };
        let hinge_rest_data = if k == 0 { vec![0.0f32] } else { reordered_hinge_rest };
        let hinge_compliance_data = if k == 0 { vec![0.0f32] } else { reordered_hinge_compliance };
        let hinges_buf = storage_buffer(&device, bytemuck::cast_slice(&hinge_hinges_data), "hinges", false);
        let hinge_rest_buf = storage_buffer(&device, bytemuck::cast_slice(&hinge_rest_data), "hinge_rest", false);
        let hinge_compliance_buf = storage_buffer(&device, bytemuck::cast_slice(&hinge_compliance_data), "hinge_compliance", false);
        let hinge_lambda_buf = storage_buffer(&device, &vec![0u8; (k * 4).max(4)], "hinge_lambda", false);
        let hinge_color_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hinge_color"),
            size: (MAX_COLORS * SLOT) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&hinge_color_buf, 0, &hinge_color_bytes);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (n * 16) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let layouts = create_layouts(&device);
        let pipelines = create_pipelines(&device, &layouts);
        let bg = build_bind_groups(
            &device, &layouts, &state, &pin, &vert_attr, &constraints, &pairs, &contact_out,
            &hash_head, &hash_next, &rest_pos, &collider_pos, &index_data, &params_buf, &color_buf,
            &hinges_buf, &hinge_rest_buf, &hinge_compliance_buf, &hinge_lambda_buf, &hinge_color_buf,
        );

        let mut solver = Solver {
            device,
            queue,
            state,
            pin,
            vert_attr,
            constraints,
            pairs,
            contact_out,
            hash_head,
            hash_next,
            rest_pos,
            collider_pos,
            index_data,
            params_buf,
            color_buf,
            hinges: hinges_buf,
            hinge_rest: hinge_rest_buf,
            hinge_compliance: hinge_compliance_buf,
            hinge_lambda: hinge_lambda_buf,
            hinge_color_buf,
            staging,
            pipelines,
            layouts,
            bg,
            n: n as u32,
            m: m as u32,
            t: t as u32,
            k: k as u32,
            n_collider_tris,
            n_colors: coloring.n_colors,
            color_counts: coloring.counts,
            hinge_n_colors: hinge_coloring.n_colors,
            hinge_color_counts: hinge_coloring.counts,
            hash_s: hash_s as u32,
            hash_mask,
            hash_min,
            hash_cell_size,
            inv_hash_cell_size,
            grid,
            tris_offset,
            cell_start_offset,
            cell_items_offset,
            collider_tris_offset,
            params: *params,
            vert_attr_host,
            cloth_tris_host: triangles.to_vec(),
            cached: positions.to_vec(),
        };
        solver.upload_all_params();
        Ok(solver)
    }

    pub fn vertex_count(&self) -> usize {
        self.n as usize
    }

    pub fn set_collider(&mut self, positions: &[f32], triangles: &[u32]) {
        let grid = build_grid(positions, triangles, self.params.body_band);
        let (index_data, tris_offset, cell_start_offset, cell_items_offset, collider_tris_offset) =
            build_index_data(&self.cloth_tris_host, &grid, triangles);
        let collider_pos_data: Vec<[f32; 4]> = positions
            .chunks_exact(3)
            .map(|p| [p[0], p[1], p[2], 0.0])
            .collect();
        let collider_data = if collider_pos_data.is_empty() {
            vec![[0.0f32; 4]]
        } else {
            collider_pos_data
        };
        self.collider_pos = storage_buffer(&self.device, bytemuck::cast_slice(&collider_data), "collider_pos", false);
        self.index_data = storage_buffer(&self.device, bytemuck::cast_slice(&index_data), "index_data", false);
        self.grid = grid;
        self.tris_offset = tris_offset;
        self.cell_start_offset = cell_start_offset;
        self.cell_items_offset = cell_items_offset;
        self.collider_tris_offset = collider_tris_offset;
        self.n_collider_tris = (triangles.len() / 3) as u32;
        self.bg = build_bind_groups(
            &self.device, &self.layouts, &self.state, &self.pin, &self.vert_attr, &self.constraints,
            &self.pairs, &self.contact_out, &self.hash_head, &self.hash_next, &self.rest_pos,
            &self.collider_pos, &self.index_data, &self.params_buf, &self.color_buf,
            &self.hinges, &self.hinge_rest, &self.hinge_compliance, &self.hinge_lambda, &self.hinge_color_buf,
        );
        self.upload_all_params();
    }

    pub fn set_pins(&mut self, mask: &[u8], targets: &[f32]) {
        let n = self.n as usize;
        let mut pin_data = vec![[0.0f32; 4]; n];
        for i in 0..n {
            pin_data[i] = [targets[3 * i], targets[3 * i + 1], targets[3 * i + 2], 0.0];
            self.vert_attr_host[i].pinned = mask[i] as u32;
        }
        self.queue.write_buffer(&self.pin, 0, bytemuck::cast_slice(&pin_data));
        self.queue
            .write_buffer(&self.vert_attr, 0, bytemuck::cast_slice(&self.vert_attr_host));
    }

    pub fn set_params(&mut self, params: SolverParams) {
        self.params = params;
        self.upload_all_params();
    }

    pub fn params(&self) -> &SolverParams {
        &self.params
    }

    pub fn positions(&self) -> Vec<f32> {
        self.cached.clone()
    }

    fn upload_all_params(&mut self) {
        let mut bytes = vec![0u8; (MAX_SUBSTEPS * SLOT) as usize];
        for s in 0..MAX_SUBSTEPS {
            let h = self.params.frame_time / (s + 1) as f32;
            let p = self.build_params(h, 0.0, 0.0);
            let base = (s * SLOT) as usize;
            bytes[base..base + std::mem::size_of::<Params>()].copy_from_slice(bytemuck::bytes_of(&p));
        }
        self.queue.write_buffer(&self.params_buf, 0, &bytes);
    }

    fn build_params(&self, h: f32, ramp: f32, gravity_ramp: f32) -> Params {
        Params {
            h,
            damping: self.params.damping,
            max_step: self.params.max_step,
            gravity_ramp,
            ramp,
            body_margin: self.params.body_margin,
            body_grip: self.params.body_grip,
            body_step: self.params.body_step,
            body_band: self.params.body_band,
            self_thickness: self.params.self_thickness,
            weld_reach: self.params.weld_reach,
            weld_gain: self.params.weld_gain,
            h2_inv: 1.0 / (h * h),
            n: self.n,
            m: self.m,
            t: self.t,
            cells: self.grid.cell_count as u32,
            n_collider_tris: self.n_collider_tris,
            hash_mask: self.hash_mask,
            hash_s: self.hash_s,
            tris_offset: self.tris_offset,
            cell_start_offset: self.cell_start_offset,
            cell_items_offset: self.cell_items_offset,
            collider_tris_offset: self.collider_tris_offset,
            gravity: [self.params.gravity[0], self.params.gravity[1], self.params.gravity[2], 0.0],
            grid_min: [self.grid.min[0], self.grid.min[1], self.grid.min[2], 0.0],
            hash_min: self.hash_min,
            grid_dims: [self.grid.dims[0], self.grid.dims[1], self.grid.dims[2], 0],
            grid_cell_size: self.grid.cell_size,
            hash_cell_size: self.hash_cell_size,
            inv_cell_size: 1.0 / self.grid.cell_size,
            inv_hash_cell_size: self.inv_hash_cell_size,
            k: self.k,
            pad: [0.0; 3],
        }
    }

    pub fn step(&mut self, substeps: u32, iterations: u32) -> Result<(), String> {
        if substeps == 0 || substeps > MAX_SUBSTEPS {
            return Err("substeps out of range".to_string());
        }
        let h = self.params.frame_time / substeps as f32;
        let base_elapsed = self.params.elapsed;
        let mut bytes = vec![0u8; (MAX_SUBSTEPS * SLOT) as usize];
        for s in 0..substeps {
            let e = base_elapsed + h * (s as f32 + 1.0);
            let ramp = (e / self.params.seam_seconds).clamp(0.0, 1.0);
            let gravity_ramp = (e / self.params.seam_seconds - 1.0).clamp(0.0, 1.0);
            let p = self.build_params(h, ramp, gravity_ramp);
            let base = (s * SLOT) as usize;
            bytes[base..base + std::mem::size_of::<Params>()].copy_from_slice(bytemuck::bytes_of(&p));
        }
        self.queue.write_buffer(&self.params_buf, 0, &bytes);

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let clear_n = self.m.max(self.n).max(self.hash_s).max(self.t).max(self.k);
        for s in 0..substeps {
            let param_off = (s * SLOT) as u32;
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipelines.clear);
            pass.set_bind_group(0, &self.bg.clear, &[param_off]);
            pass.dispatch_workgroups(div_ceil(clear_n, WORKGROUP), 1, 1);
            pass.set_pipeline(&self.pipelines.integrate);
            pass.set_bind_group(0, &self.bg.integrate, &[param_off]);
            pass.dispatch_workgroups(div_ceil(self.n, WORKGROUP), 1, 1);
            for it in 0..iterations {
                pass.set_pipeline(&self.pipelines.pairs);
                for c in 0..self.n_colors {
                    pass.set_bind_group(0, &self.bg.pairs, &[param_off, (c * SLOT) as u32]);
                    pass.dispatch_workgroups(div_ceil(self.color_count(c), WORKGROUP), 1, 1);
                }
                if it % 2 == 0 && self.hinge_n_colors > 0 {
                    pass.set_pipeline(&self.pipelines.hinges);
                    for c in 0..self.hinge_n_colors {
                        pass.set_bind_group(0, &self.bg.hinges, &[param_off, (c * SLOT) as u32]);
                        pass.dispatch_workgroups(div_ceil(self.hinge_color_counts[c as usize], WORKGROUP), 1, 1);
                    }
                }
            }
            if self.params.self_thickness > 0.0 {
                pass.set_pipeline(&self.pipelines.hash);
                pass.set_bind_group(0, &self.bg.hash, &[param_off]);
                pass.dispatch_workgroups(div_ceil(self.t, WORKGROUP), 1, 1);
            }
            pass.set_pipeline(&self.pipelines.contacts);
            pass.set_bind_group(0, &self.bg.contacts, &[param_off]);
            pass.dispatch_workgroups(div_ceil(self.n, WORKGROUP), 1, 1);
            pass.set_pipeline(&self.pipelines.apply);
            pass.set_bind_group(0, &self.bg.apply, &[param_off]);
            pass.dispatch_workgroups(div_ceil(self.n, WORKGROUP), 1, 1);
            pass.set_pipeline(&self.pipelines.finish);
            pass.set_bind_group(0, &self.bg.finish, &[param_off]);
            pass.dispatch_workgroups(div_ceil(self.n, WORKGROUP), 1, 1);
        }
        self.queue.submit(Some(encoder.finish()));
        self.readback()
    }

    fn color_count(&self, color: u32) -> u32 {
        self.color_counts[color as usize]
    }

    fn readback(&mut self) -> Result<(), String> {
        let n = self.n as usize;
        let n_bytes = (n * 16) as u64;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(&self.state, 0, &self.staging, 0, n_bytes);
        self.queue.submit(Some(encoder.finish()));
        {
            let slice = self.staging.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            match rx.recv() {
                Ok(Ok(())) => {}
                _ => return Err("GPU device lost".to_string()),
            }
            let view = slice.get_mapped_range().map_err(|_| "GPU device lost".to_string())?;
            let data: &[u8] = &view;
            for i in 0..n {
                let base = i * 16;
                self.cached[3 * i] = f32::from_le_bytes(data[base..base + 4].try_into().unwrap());
                self.cached[3 * i + 1] = f32::from_le_bytes(data[base + 4..base + 8].try_into().unwrap());
                self.cached[3 * i + 2] = f32::from_le_bytes(data[base + 8..base + 12].try_into().unwrap());
            }
            drop(view);
            self.staging.unmap();
        }
        Ok(())
    }
}

fn max_tri_radius(triangles: &[u32], rest_positions: &[f32]) -> f32 {
    let mut r = 0.0f32;
    for tri in triangles.chunks_exact(3) {
        let a = [rest_positions[tri[0] as usize * 3], rest_positions[tri[0] as usize * 3 + 1], rest_positions[tri[0] as usize * 3 + 2]];
        let b = [rest_positions[tri[1] as usize * 3], rest_positions[tri[1] as usize * 3 + 1], rest_positions[tri[1] as usize * 3 + 2]];
        let c = [rest_positions[tri[2] as usize * 3], rest_positions[tri[2] as usize * 3 + 1], rest_positions[tri[2] as usize * 3 + 2]];
        let centroid = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        for v in [a, b, c] {
            let dx = v[0] - centroid[0];
            let dy = v[1] - centroid[1];
            let dz = v[2] - centroid[2];
            r = r.max((dx * dx + dy * dy + dz * dz).sqrt());
        }
    }
    r
}

fn build_index_data(cloth_tris: &[u32], grid: &Grid, collider_tris: &[u32]) -> (Vec<u32>, u32, u32, u32, u32) {
    let tris_offset = 0u32;
    let cell_start_offset = cloth_tris.len() as u32;
    let cell_items_offset = cell_start_offset + grid.cell_start.len() as u32;
    let collider_tris_offset = cell_items_offset + grid.cell_items.len() as u32;
    let mut out = Vec::with_capacity(
        cloth_tris.len() + grid.cell_start.len() + grid.cell_items.len() + collider_tris.len(),
    );
    out.extend_from_slice(cloth_tris);
    out.extend_from_slice(&grid.cell_start);
    out.extend_from_slice(&grid.cell_items);
    out.extend_from_slice(collider_tris);
    (out, tris_offset, cell_start_offset, cell_items_offset, collider_tris_offset)
}
