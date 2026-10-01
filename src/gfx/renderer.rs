//! Forward HDR renderer: shadow map, sky, PBR scene, bloom, composite and UI.

use super::gpu::Gpu;
use super::mesh::{Instance, MeshData, SkinMeshData, SkinVertex, Vertex};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};

pub const MAX_BONES: usize = 40;
pub const MAX_LIGHTS: usize = 8;
const BLOOM_LEVELS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MeshId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawPass {
    Opaque,
    Transparent,
    Additive,
}

#[derive(Clone, Copy, Debug)]
pub struct Draw {
    pub mesh: MeshId,
    pub xf: Mat4,
    pub tint: Vec4,
    pub params: Vec4,
    pub pass: DrawPass,
    pub shadow: bool,
    pub main: bool,
    pub palette: u32,
}

impl Draw {
    pub fn new(mesh: MeshId, xf: Mat4) -> Self {
        Draw {
            mesh,
            xf,
            tint: Vec4::ONE,
            params: Vec4::ZERO,
            pass: DrawPass::Opaque,
            shadow: true,
            main: true,
            palette: u32::MAX,
        }
    }
    pub fn tint(mut self, t: Vec4) -> Self {
        self.tint = t;
        self
    }
    pub fn params(mut self, p: Vec4) -> Self {
        self.params = p;
        self
    }
    pub fn pass(mut self, p: DrawPass) -> Self {
        self.pass = p;
        if p != DrawPass::Opaque {
            self.shadow = false;
        }
        self
    }
    pub fn no_shadow(mut self) -> Self {
        self.shadow = false;
        self
    }
    pub fn shadow_only(mut self) -> Self {
        self.main = false;
        self
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct PointLightRaw {
    pub pos: [f32; 4],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GlobalsRaw {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    shadow_mat: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky_up: [f32; 4],
    sky_down: [f32; 4],
    fog: [f32; 4],
    sky_zenith: [f32; 4],
    sky_horizon: [f32; 4],
    params: [f32; 4],
    rim: [f32; 4],
    lights: [PointLightRaw; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostRaw {
    a: [f32; 4],
    b: [f32; 4],
    tint: [f32; 4],
    lift: [f32; 4],
    c: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UiGlobalsRaw {
    screen: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Default)]
pub struct UiVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub local: [f32; 2],
    pub size: [f32; 2],
    pub color: [u8; 4],
    pub params: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct UiCmd {
    pub start: u32,
    pub count: u32,
    pub clip: Option<[f32; 4]>,
}

#[derive(Default)]
pub struct UiBatch {
    pub verts: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub cmds: Vec<UiCmd>,
    /// Pending atlas uploads: (x, y, w, h, pixels)
    pub atlas_uploads: Vec<(u32, u32, u32, u32, Vec<u8>)>,
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub pos: Vec3,
    pub radius: f32,
    pub color: Vec3,
}

#[derive(Clone, Debug)]
pub struct SceneParams {
    pub cam_pos: Vec3,
    pub view: Mat4,
    pub proj: Mat4,
    pub time: f32,
    pub sun_dir: Vec3,
    pub sun_color: Vec3,
    pub shadow_strength: f32,
    pub sun_disc: f32,
    pub sky_up: Vec3,
    pub sky_down: Vec3,
    pub ambient: f32,
    pub env_spec: f32,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub zenith: Vec3,
    pub horizon: Vec3,
    pub clouds: f32,
    pub stars: f32,
    pub rim_color: Vec3,
    pub rim_strength: f32,
    pub lights: Vec<Light>,
    pub shadow_center: Vec3,
    pub shadow_radius: f32,
    pub draw_sky: bool,
    pub clear_color: Vec3,
}

impl Default for SceneParams {
    fn default() -> Self {
        SceneParams {
            cam_pos: Vec3::new(0.0, 1.6, 4.0),
            view: Mat4::IDENTITY,
            proj: Mat4::IDENTITY,
            time: 0.0,
            sun_dir: Vec3::new(0.3, 0.8, 0.4).normalize(),
            sun_color: Vec3::splat(3.0),
            shadow_strength: 1.0,
            sun_disc: 1.0,
            sky_up: Vec3::new(0.4, 0.5, 0.7),
            sky_down: Vec3::new(0.2, 0.18, 0.15),
            ambient: 1.0,
            env_spec: 1.0,
            fog_color: Vec3::new(0.6, 0.7, 0.8),
            fog_density: 0.0,
            zenith: Vec3::new(0.2, 0.4, 0.8),
            horizon: Vec3::new(0.7, 0.8, 0.9),
            clouds: 0.4,
            stars: 0.0,
            rim_color: Vec3::new(1.0, 0.9, 0.8),
            rim_strength: 0.0,
            lights: Vec::new(),
            shadow_center: Vec3::ZERO,
            shadow_radius: 6.0,
            draw_sky: true,
            clear_color: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PostSettings {
    pub exposure: f32,
    pub bloom: f32,
    pub bloom_threshold: f32,
    pub vignette: f32,
    pub grain: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub tint: Vec3,
    pub lift: Vec3,
    pub fade: f32,
    pub letterbox: f32,
    pub desaturate: f32,
    pub flash: f32,
    pub aberration: f32,
}

impl Default for PostSettings {
    fn default() -> Self {
        PostSettings {
            exposure: 1.0,
            bloom: 0.08,
            bloom_threshold: 1.2,
            vignette: 0.35,
            grain: 0.018,
            saturation: 1.05,
            contrast: 1.04,
            tint: Vec3::ONE,
            lift: Vec3::ZERO,
            fade: 0.0,
            letterbox: 0.0,
            desaturate: 0.0,
            flash: 0.0,
            aberration: 0.4,
        }
    }
}

/// Draw call and triangle counts of the last frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub opaque: u32,
    pub shadow: u32,
    pub transparent: u32,
    pub tris_main: u32,
    pub tris_shadow: u32,
}

pub struct FrameScene {
    pub params: SceneParams,
    pub post: PostSettings,
    pub draws: Vec<Draw>,
    pub palettes: Vec<[Mat4; MAX_BONES]>,
}

impl FrameScene {
    pub fn new() -> Self {
        FrameScene {
            params: SceneParams::default(),
            post: PostSettings::default(),
            draws: Vec::new(),
            palettes: Vec::new(),
        }
    }
}

impl Default for FrameScene {
    fn default() -> Self {
        Self::new()
    }
}

struct GpuMesh {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    index_count: u32,
    vcap: u64,
    icap: u64,
    skinned: bool,
}

struct Targets {
    width: u32,
    height: u32,
    msaa_color: Option<wgpu::TextureView>,
    hdr: wgpu::TextureView,
    depth: wgpu::TextureView,
    bloom: Vec<wgpu::TextureView>,
    glass: Vec<wgpu::TextureView>,
    bloom_down_bg: Vec<wgpu::BindGroup>,
    bloom_up_bg: Vec<wgpu::BindGroup>,
    glass_bg: Vec<wgpu::BindGroup>,
    composite_bg: wgpu::BindGroup,
    ui_bg: wgpu::BindGroup,
}

struct Group {
    mesh: MeshId,
    palette: u32,
    offset: u64,
    count: u32,
}

pub struct Renderer {
    pub surface_format: wgpu::TextureFormat,
    hdr_format: wgpu::TextureFormat,
    msaa: u32,
    globals_buf: wgpu::Buffer,
    main_bg: wgpu::BindGroup,
    shadow_bg: wgpu::BindGroup,
    bones_buf: wgpu::Buffer,
    bones_bg: wgpu::BindGroup,
    bone_stride: u64,
    max_palettes: usize,
    instance_buf: wgpu::Buffer,
    instance_cap: u64,
    p_static: wgpu::RenderPipeline,
    p_skin: wgpu::RenderPipeline,
    p_shadow_static: wgpu::RenderPipeline,
    p_shadow_skin: wgpu::RenderPipeline,
    p_transparent: wgpu::RenderPipeline,
    p_transparent_skin: wgpu::RenderPipeline,
    p_additive: wgpu::RenderPipeline,
    p_sky: wgpu::RenderPipeline,
    shadow_view: wgpu::TextureView,
    shadow_size: u32,
    post_bgl: wgpu::BindGroupLayout,
    post_buf: wgpu::Buffer,
    p_down_first: wgpu::RenderPipeline,
    p_down: wgpu::RenderPipeline,
    p_up: wgpu::RenderPipeline,
    p_composite: wgpu::RenderPipeline,
    linear_smp: wgpu::Sampler,
    targets: Option<Targets>,
    meshes: Vec<Option<GpuMesh>>,
    free_ids: Vec<u32>,
    // ui
    ui_bgl: wgpu::BindGroupLayout,
    ui_pipeline: wgpu::RenderPipeline,
    ui_globals: wgpu::Buffer,
    ui_vbuf: wgpu::Buffer,
    ui_ibuf: wgpu::Buffer,
    ui_vcap: u64,
    ui_icap: u64,
    atlas: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    pub atlas_size: u32,
    pub render_scale: f32,
    /// Phones: fewer shadow taps and point lights per pixel.
    low: bool,
    pub stats: FrameStats,
    pub last_view_proj: Mat4,
}

fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Unorm8x4
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

fn skin_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Unorm8x4, 10 => Uint8x4, 11 => Unorm8x4
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SkinVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

fn instance_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Instance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &ATTRS,
    }
}

fn ui_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x2, 4 => Unorm8x4, 5 => Float32x4
    ];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<UiVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRS,
    }
}

fn uniform_entry(binding: u32, vis: wgpu::ShaderStages, dynamic: bool, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: dynamic,
            min_binding_size: wgpu::BufferSize::new(size),
        },
        count: None,
    }
}

fn tex_entry(binding: u32, sample_type: wgpu::TextureSampleType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32, ty: wgpu::SamplerBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(ty),
        count: None,
    }
}

impl Renderer {
    pub fn new(gpu: &Gpu, surface_format: wgpu::TextureFormat) -> Renderer {
        let device = &gpu.device;
        let hdr_format = gpu.info.hdr_format;
        let msaa = gpu.info.msaa;

        let scene_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/post.wgsl").into()),
        });
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()),
        });

        let gsize = std::mem::size_of::<GlobalsRaw>() as u64;
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: gsize,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let main_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("main_bgl"),
            entries: &[
                uniform_entry(0, vf, false, gsize),
                tex_entry(1, wgpu::TextureSampleType::Depth),
                sampler_entry(2, wgpu::SamplerBindingType::Comparison),
            ],
        });
        let shadow_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_bgl"),
            entries: &[uniform_entry(0, vf, false, gsize)],
        });

        let bone_bytes = (MAX_BONES * 64) as u64;
        let align = device.limits().min_uniform_buffer_offset_alignment as u64;
        let bone_stride = bone_bytes.div_ceil(align) * align;
        let max_palettes = 24usize;
        let bones_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bones"),
            size: bone_stride * max_palettes as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bones_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bones_bgl"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX, true, bone_bytes)],
        });
        let bones_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bones_bg"),
            layout: &bones_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &bones_buf,
                    offset: 0,
                    size: wgpu::BufferSize::new(bone_bytes),
                }),
            }],
        });

        // shadow map
        let shadow_size = gpu.info.shadow_size;
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow"),
            size: wgpu::Extent3d {
                width: shadow_size,
                height: shadow_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_smp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_smp"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let main_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("main_bg"),
            layout: &main_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_smp),
                },
            ],
        });
        let shadow_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_bg"),
            layout: &shadow_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buf.as_entire_binding(),
            }],
        });

        let static_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("static_layout"),
            bind_group_layouts: &[Some(&main_bgl)],
            immediate_size: 0,
        });
        let skin_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skin_layout"),
            bind_group_layouts: &[Some(&main_bgl), Some(&bones_bgl)],
            immediate_size: 0,
        });
        let shadow_static_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_static_layout"),
            bind_group_layouts: &[Some(&shadow_bgl)],
            immediate_size: 0,
        });
        let shadow_skin_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_skin_layout"),
            bind_group_layouts: &[Some(&shadow_bgl), Some(&bones_bgl)],
            immediate_size: 0,
        });

        let depth_state = |write: bool, cmp: wgpu::CompareFunction| wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(write),
            depth_compare: Some(cmp),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };
        let ms = wgpu::MultisampleState {
            count: msaa,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let make_scene = |label: &str,
                          layout: &wgpu::PipelineLayout,
                          vs: &str,
                          fs: &str,
                          vlayout: wgpu::VertexBufferLayout<'static>,
                          blend: Option<wgpu::BlendState>,
                          depth_write: bool,
                          cull: Option<wgpu::Face>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &scene_shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(vlayout), Some(instance_layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &scene_shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: hdr_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: cull,
                    ..Default::default()
                },
                depth_stencil: Some(depth_state(depth_write, wgpu::CompareFunction::LessEqual)),
                multisample: ms,
                multiview_mask: None,
                cache: None,
            })
        };

        let premul = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let p_static = make_scene("static", &static_layout, "vs_static", "fs_main", vertex_layout(), None, true, Some(wgpu::Face::Back));
        let p_skin = make_scene("skin", &skin_layout, "vs_skin", "fs_main", skin_vertex_layout(), None, true, Some(wgpu::Face::Back));
        let p_transparent = make_scene("transparent", &static_layout, "vs_static", "fs_transparent", vertex_layout(), Some(premul), false, None);
        let p_transparent_skin = make_scene("transparent_skin", &skin_layout, "vs_skin", "fs_transparent", skin_vertex_layout(), Some(premul), false, Some(wgpu::Face::Back));
        let p_additive = make_scene("additive", &static_layout, "vs_static", "fs_additive", vertex_layout(), Some(additive), false, None);

        let make_shadow = |label: &str, layout: &wgpu::PipelineLayout, vs: &str, vlayout: wgpu::VertexBufferLayout<'static>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &scene_shader,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(vlayout), Some(instance_layout())],
                },
                fragment: None,
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let p_shadow_static = make_shadow("shadow_static", &shadow_static_layout, "vs_shadow_static", vertex_layout());
        let p_shadow_skin = make_shadow("shadow_skin", &shadow_skin_layout, "vs_shadow_skin", skin_vertex_layout());

        let p_sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&static_layout),
            vertex: wgpu::VertexState {
                module: &scene_shader,
                entry_point: Some("vs_sky"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &scene_shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: hdr_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(depth_state(false, wgpu::CompareFunction::Always)),
            multisample: ms,
            multiview_mask: None,
            cache: None,
        });

        // post
        let psize = std::mem::size_of::<PostRaw>() as u64;
        let post_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: psize,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let float_tex = wgpu::TextureSampleType::Float { filterable: true };
        let post_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post_bgl"),
            entries: &[
                tex_entry(0, float_tex),
                sampler_entry(1, wgpu::SamplerBindingType::Filtering),
                uniform_entry(2, wgpu::ShaderStages::FRAGMENT, false, psize),
                tex_entry(3, float_tex),
            ],
        });
        let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post_layout"),
            bind_group_layouts: &[Some(&post_bgl)],
            immediate_size: 0,
        });
        let make_post = |label: &str, fs: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&post_layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let p_down_first = make_post("bloom_down_first", "fs_down_first", hdr_format, None);
        let p_down = make_post("bloom_down", "fs_down", hdr_format, None);
        let p_up = make_post("bloom_up", "fs_up", hdr_format, Some(additive));
        let p_composite = make_post("composite", "fs_composite", surface_format, None);
        let linear_smp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // UI
        let usize_ = std::mem::size_of::<UiGlobalsRaw>() as u64;
        let ui_globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_globals"),
            size: usize_,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui_bgl"),
            entries: &[
                uniform_entry(0, vf, false, usize_),
                tex_entry(1, float_tex),
                tex_entry(2, float_tex),
                sampler_entry(3, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let ui_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui_layout"),
            bind_group_layouts: &[Some(&ui_bgl)],
            immediate_size: 0,
        });
        let ui_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui"),
            layout: Some(&ui_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_ui"),
                compilation_options: Default::default(),
                buffers: &[Some(ui_vertex_layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_ui"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(ui_blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let atlas_size = 1024u32;
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size: wgpu::Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let ui_vcap = 1 << 16;
        let ui_icap = 1 << 17;
        let ui_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_v"),
            size: ui_vcap * std::mem::size_of::<UiVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_i"),
            size: ui_icap * 4,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_cap = 4096u64;
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: instance_cap * std::mem::size_of::<Instance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Renderer {
            surface_format,
            hdr_format,
            msaa,
            globals_buf,
            main_bg,
            shadow_bg,
            bones_buf,
            bones_bg,
            bone_stride,
            max_palettes,
            instance_buf,
            instance_cap,
            p_static,
            p_skin,
            p_shadow_static,
            p_shadow_skin,
            p_transparent,
            p_transparent_skin,
            p_additive,
            p_sky,
            shadow_view,
            shadow_size,
            post_bgl,
            post_buf,
            p_down_first,
            p_down,
            p_up,
            p_composite,
            linear_smp,
            targets: None,
            meshes: Vec::new(),
            free_ids: Vec::new(),
            ui_bgl,
            ui_pipeline,
            ui_globals,
            ui_vbuf,
            ui_ibuf,
            ui_vcap,
            ui_icap,
            atlas,
            atlas_view,
            atlas_size,
            render_scale: gpu.info.render_scale,
            low: gpu.info.is_mobile,
            stats: FrameStats::default(),
            last_view_proj: Mat4::IDENTITY,
        }
    }

    // ------------------------------------------------------------ meshes

    fn alloc_id(&mut self) -> u32 {
        if let Some(id) = self.free_ids.pop() {
            id
        } else {
            self.meshes.push(None);
            (self.meshes.len() - 1) as u32
        }
    }

    pub fn upload_mesh(&mut self, gpu: &Gpu, data: &MeshData) -> MeshId {
        let id = self.alloc_id();
        self.meshes[id as usize] = Some(Self::create_gpu_mesh(
            gpu,
            bytemuck::cast_slice(&data.verts),
            &data.indices,
            false,
        ));
        MeshId(id)
    }

    pub fn upload_skinned(&mut self, gpu: &Gpu, data: &SkinMeshData) -> MeshId {
        let id = self.alloc_id();
        self.meshes[id as usize] = Some(Self::create_gpu_mesh(
            gpu,
            bytemuck::cast_slice(&data.verts),
            &data.indices,
            true,
        ));
        MeshId(id)
    }

    fn create_gpu_mesh(gpu: &Gpu, vbytes: &[u8], indices: &[u32], skinned: bool) -> GpuMesh {
        let vdata: Vec<u8> = if vbytes.is_empty() { vec![0u8; 64] } else { vbytes.to_vec() };
        let idata: Vec<u32> = if indices.is_empty() { vec![0u32; 4] } else { indices.to_vec() };
        // Avoid mappedAtCreation: some WebGPU implementations cap its size.
        let vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_v"),
            size: (vdata.len() as u64).div_ceil(4) * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(&vbuf, 0, &vdata);
        let ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_i"),
            size: (idata.len() * 4) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(&ibuf, 0, bytemuck::cast_slice(&idata));
        GpuMesh {
            vbuf,
            ibuf,
            index_count: indices.len() as u32,
            vcap: vdata.len() as u64,
            icap: (idata.len() * 4) as u64,
            skinned,
        }
    }

    /// Replaces the contents of a dynamic static-vertex mesh.
    pub fn update_mesh(&mut self, gpu: &Gpu, id: MeshId, data: &MeshData) {
        let vbytes: &[u8] = bytemuck::cast_slice(&data.verts);
        let ibytes: &[u8] = bytemuck::cast_slice(&data.indices);
        if let Some(Some(m)) = self.meshes.get_mut(id.0 as usize) {
            if (vbytes.len() as u64) <= m.vcap && (ibytes.len() as u64) <= m.icap && !vbytes.is_empty() {
                gpu.queue.write_buffer(&m.vbuf, 0, vbytes);
                gpu.queue.write_buffer(&m.ibuf, 0, ibytes);
                m.index_count = data.indices.len() as u32;
                return;
            }
        }
        let mut nm = Self::create_gpu_mesh(gpu, vbytes, &data.indices, false);
        // over-allocate to reduce reallocations
        if !vbytes.is_empty() {
            let vb = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dyn_v"),
                size: (vbytes.len() as u64 * 2).max(256),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let ib = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("dyn_i"),
                size: (ibytes.len() as u64 * 2).max(256),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            gpu.queue.write_buffer(&vb, 0, vbytes);
            gpu.queue.write_buffer(&ib, 0, ibytes);
            nm.vcap = vb.size();
            nm.icap = ib.size();
            nm.vbuf = vb;
            nm.ibuf = ib;
        }
        if (id.0 as usize) < self.meshes.len() {
            self.meshes[id.0 as usize] = Some(nm);
        }
    }

    pub fn free_mesh(&mut self, id: MeshId) {
        if let Some(slot) = self.meshes.get_mut(id.0 as usize) {
            if slot.take().is_some() {
                self.free_ids.push(id.0);
            }
        }
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.iter().filter(|m| m.is_some()).count()
    }

    // ------------------------------------------------------------ targets

    fn ensure_targets(&mut self, gpu: &Gpu, width: u32, height: u32) {
        let w = ((width as f32 * self.render_scale) as u32).max(16);
        let h = ((height as f32 * self.render_scale) as u32).max(16);
        if let Some(t) = &self.targets {
            if t.width == w && t.height == h {
                return;
            }
        }
        let device = &gpu.device;
        let mk = |label: &str, w: u32, h: u32, format: wgpu::TextureFormat, samples: u32, usage: wgpu::TextureUsages| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: w.max(1),
                        height: h.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let rt = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let rts = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let msaa_color = if self.msaa > 1 {
            Some(mk("hdr_msaa", w, h, self.hdr_format, self.msaa, rt))
        } else {
            None
        };
        let hdr = mk("hdr", w, h, self.hdr_format, 1, rts);
        let depth = mk("depth", w, h, wgpu::TextureFormat::Depth32Float, self.msaa, rt);
        let mut bloom = Vec::new();
        let (mut bw, mut bh) = (w, h);
        for i in 0..BLOOM_LEVELS {
            bw = (bw / 2).max(1);
            bh = (bh / 2).max(1);
            bloom.push(mk(&format!("bloom{i}"), bw, bh, self.hdr_format, 1, rts));
        }
        let glass = vec![
            mk("glass0", (w / 4).max(1), (h / 4).max(1), self.hdr_format, 1, rts),
            mk("glass1", (w / 8).max(1), (h / 8).max(1), self.hdr_format, 1, rts),
            mk("glass2", (w / 16).max(1), (h / 16).max(1), self.hdr_format, 1, rts),
        ];
        let post_bg = |src: &wgpu::TextureView, extra: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("post_bg"),
                layout: &self.post_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.linear_smp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.post_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(extra),
                    },
                ],
            })
        };
        let mut bloom_down_bg = Vec::new();
        for i in 0..BLOOM_LEVELS {
            let src = if i == 0 { &hdr } else { &bloom[i - 1] };
            bloom_down_bg.push(post_bg(src, &glass[0]));
        }
        let mut bloom_up_bg = Vec::new();
        for i in 0..BLOOM_LEVELS - 1 {
            // writes into bloom[i], reads bloom[i+1]
            bloom_up_bg.push(post_bg(&bloom[i + 1], &glass[0]));
        }
        let glass_bg = vec![
            post_bg(&hdr, &bloom[0]),
            post_bg(&glass[0], &bloom[0]),
            post_bg(&glass[1], &bloom[0]),
        ];
        let composite_bg = post_bg(&hdr, &bloom[0]);
        let ui_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui_bg"),
            layout: &self.ui_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.ui_globals.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&glass[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.linear_smp),
                },
            ],
        });
        self.targets = Some(Targets {
            width: w,
            height: h,
            msaa_color,
            hdr,
            depth,
            bloom,
            glass,
            bloom_down_bg,
            bloom_up_bg,
            glass_bg,
            composite_bg,
            ui_bg,
        });
    }

    pub fn set_render_scale(&mut self, s: f32) {
        let s = s.clamp(0.4, 2.0);
        if (s - self.render_scale).abs() > 1e-3 {
            self.render_scale = s;
            self.targets = None;
        }
    }

    // ------------------------------------------------------------ frame

    fn shadow_matrix(p: &SceneParams, shadow_size: u32) -> Mat4 {
        let dir = p.sun_dir.normalize_or(Vec3::Y);
        let r = p.shadow_radius.max(1.0);
        let up = if dir.y.abs() > 0.95 { Vec3::Z } else { Vec3::Y };
        let eye = p.shadow_center + dir * r * 3.0;
        let view = Mat4::look_at_rh(eye, p.shadow_center, up);
        // texel snapping to avoid shimmering
        let texel = 2.0 * r / shadow_size as f32;
        let c = view.transform_point3(p.shadow_center);
        let snapped = Vec3::new((c.x / texel).round() * texel, (c.y / texel).round() * texel, c.z);
        let fix = Mat4::from_translation(Vec3::new(snapped.x - c.x, snapped.y - c.y, 0.0));
        let proj = Mat4::orthographic_rh(-r, r, -r, r, 0.05, r * 6.0);
        proj * fix * view
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        target: &wgpu::TextureView,
        width: u32,
        height: u32,
        scene: &FrameScene,
        ui: &mut UiBatch,
    ) {
        self.ensure_targets(gpu, width, height);
        let queue = &gpu.queue;
        let p = &scene.params;

        // globals
        let view_proj = p.proj * p.view;
        self.last_view_proj = view_proj;
        let shadow_mat = Self::shadow_matrix(p, self.shadow_size);
        let mut lights = [PointLightRaw::default(); MAX_LIGHTS];
        // choose the lights closest to the camera
        let mut ls: Vec<&Light> = p.lights.iter().collect();
        ls.sort_by(|a, b| {
            let da = a.pos.distance_squared(p.cam_pos) / (a.radius * a.radius);
            let db = b.pos.distance_squared(p.cam_pos) / (b.radius * b.radius);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
        let nl = ls.len().min(if self.low { 4 } else { MAX_LIGHTS });
        for (i, l) in ls.iter().take(nl).enumerate() {
            lights[i] = PointLightRaw {
                pos: [l.pos.x, l.pos.y, l.pos.z, l.radius],
                color: [l.color.x, l.color.y, l.color.z, 0.0],
            };
        }
        let g = GlobalsRaw {
            view_proj: view_proj.to_cols_array_2d(),
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            shadow_mat: shadow_mat.to_cols_array_2d(),
            cam_pos: [p.cam_pos.x, p.cam_pos.y, p.cam_pos.z, p.time],
            sun_dir: [p.sun_dir.x, p.sun_dir.y, p.sun_dir.z, p.shadow_strength],
            sun_color: [p.sun_color.x, p.sun_color.y, p.sun_color.z, p.sun_disc],
            sky_up: [p.sky_up.x, p.sky_up.y, p.sky_up.z, p.ambient],
            sky_down: [p.sky_down.x, p.sky_down.y, p.sky_down.z, p.env_spec],
            fog: [p.fog_color.x, p.fog_color.y, p.fog_color.z, p.fog_density],
            sky_zenith: [p.zenith.x, p.zenith.y, p.zenith.z, p.clouds],
            sky_horizon: [p.horizon.x, p.horizon.y, p.horizon.z, p.stars],
            params: [nl as f32, 1.0 / self.shadow_size as f32, p.rim_strength, if self.low { 4.0 } else { 8.0 }],
            rim: [p.rim_color.x, p.rim_color.y, p.rim_color.z, 1.0],
            lights,
        };
        queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&g));

        // bone palettes
        let npal = scene.palettes.len().min(self.max_palettes);
        for (i, pal) in scene.palettes.iter().take(npal).enumerate() {
            let raw: Vec<[[f32; 4]; 4]> = pal.iter().map(|m| m.to_cols_array_2d()).collect();
            queue.write_buffer(&self.bones_buf, i as u64 * self.bone_stride, bytemuck::cast_slice(&raw));
        }

        // instances
        let mut inst: Vec<Instance> = Vec::with_capacity(scene.draws.len() * 2);
        let valid = |d: &Draw| -> bool {
            if d.palette != u32::MAX && d.palette as usize >= npal {
                return false;
            }
            matches!(self.meshes.get(d.mesh.0 as usize), Some(Some(m)) if m.index_count > 0)
        };
        let to_inst = |d: &Draw| Instance {
            model: d.xf.to_cols_array_2d(),
            tint: d.tint.to_array(),
            params: d.params.to_array(),
        };
        let isize = std::mem::size_of::<Instance>() as u64;
        let build_groups = |filter: &dyn Fn(&Draw) -> bool, inst: &mut Vec<Instance>| -> Vec<Group> {
            let mut idx: Vec<usize> = (0..scene.draws.len()).filter(|&i| filter(&scene.draws[i]) && valid(&scene.draws[i])).collect();
            idx.sort_by_key(|&i| (scene.draws[i].palette, scene.draws[i].mesh));
            let mut groups: Vec<Group> = Vec::new();
            for i in idx {
                let d = &scene.draws[i];
                let off = inst.len() as u64 * isize;
                inst.push(to_inst(d));
                if let Some(last) = groups.last_mut() {
                    if last.mesh == d.mesh && last.palette == d.palette && d.palette == u32::MAX {
                        last.count += 1;
                        continue;
                    }
                }
                groups.push(Group {
                    mesh: d.mesh,
                    palette: d.palette,
                    offset: off,
                    count: 1,
                });
            }
            groups
        };
        let shadow_on = p.shadow_strength > 0.0;
        let opaque = build_groups(&|d: &Draw| d.main && d.pass == DrawPass::Opaque, &mut inst);
        let shadows = if shadow_on {
            build_groups(&|d: &Draw| d.shadow, &mut inst)
        } else {
            Vec::new()
        };
        // transparent: back to front
        let mut tidx: Vec<usize> = (0..scene.draws.len())
            .filter(|&i| {
                let d = &scene.draws[i];
                d.main && d.pass == DrawPass::Transparent && valid(d)
            })
            .collect();
        tidx.sort_by(|&a, &b| {
            let da = scene.draws[a].xf.w_axis.truncate().distance_squared(p.cam_pos);
            let db = scene.draws[b].xf.w_axis.truncate().distance_squared(p.cam_pos);
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut transparent = Vec::new();
        for i in tidx {
            let d = &scene.draws[i];
            transparent.push(Group {
                mesh: d.mesh,
                palette: d.palette,
                offset: inst.len() as u64 * isize,
                count: 1,
            });
            inst.push(to_inst(d));
        }
        let additive = build_groups(&|d: &Draw| d.main && d.pass == DrawPass::Additive, &mut inst);
        let tris = |gs: &[Group]| -> u32 {
            gs.iter()
                .map(|g| self.meshes[g.mesh.0 as usize].as_ref().map(|m| m.index_count / 3 * g.count).unwrap_or(0))
                .sum()
        };
        self.stats = FrameStats {
            opaque: opaque.len() as u32,
            shadow: shadows.len() as u32,
            transparent: (transparent.len() + additive.len()) as u32,
            tris_main: tris(&opaque),
            tris_shadow: tris(&shadows),
        };

        if inst.len() as u64 > self.instance_cap {
            self.instance_cap = (inst.len() as u64).next_power_of_two();
            self.instance_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instances"),
                size: self.instance_cap * isize,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !inst.is_empty() {
            queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&inst));
        }

        // post params
        let ps = &scene.post;
        let post = PostRaw {
            a: [ps.exposure, ps.bloom, ps.vignette, ps.grain],
            b: [p.time, ps.saturation, ps.contrast, ps.bloom_threshold],
            tint: [ps.tint.x, ps.tint.y, ps.tint.z, ps.aberration],
            lift: [ps.lift.x, ps.lift.y, ps.lift.z, ps.fade],
            c: [ps.letterbox, ps.desaturate, ps.flash, 0.0],
        };
        queue.write_buffer(&self.post_buf, 0, bytemuck::bytes_of(&post));
        queue.write_buffer(
            &self.ui_globals,
            0,
            bytemuck::bytes_of(&UiGlobalsRaw {
                screen: [width as f32, height as f32, p.time, ps.exposure],
            }),
        );

        // atlas uploads
        for (x, y, w, h, data) in ui.atlas_uploads.drain(..) {
            if w == 0 || h == 0 {
                continue;
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }

        // ui buffers
        if ui.verts.len() as u64 > self.ui_vcap {
            self.ui_vcap = (ui.verts.len() as u64).next_power_of_two();
            self.ui_vbuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui_v"),
                size: self.ui_vcap * std::mem::size_of::<UiVertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if ui.indices.len() as u64 > self.ui_icap {
            self.ui_icap = (ui.indices.len() as u64).next_power_of_two();
            self.ui_ibuf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui_i"),
                size: self.ui_icap * 4,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !ui.verts.is_empty() {
            queue.write_buffer(&self.ui_vbuf, 0, bytemuck::cast_slice(&ui.verts));
            let mut idx = ui.indices.clone();
            // index buffer writes must be 4-byte aligned in size: u32 always is
            if idx.is_empty() {
                idx.push(0);
            }
            queue.write_buffer(&self.ui_ibuf, 0, bytemuck::cast_slice(&idx));
        }

        let t = self.targets.as_ref().unwrap();
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });

        let draw_groups = |pass: &mut wgpu::RenderPass<'_>,
                           groups: &[Group],
                           p_static: &wgpu::RenderPipeline,
                           p_skin: &wgpu::RenderPipeline,
                           bg0: &wgpu::BindGroup| {
            let mut cur_skin: Option<bool> = None;
            for gr in groups {
                let Some(Some(mesh)) = self.meshes.get(gr.mesh.0 as usize) else {
                    continue;
                };
                let skinned = mesh.skinned && gr.palette != u32::MAX;
                if mesh.skinned && gr.palette == u32::MAX {
                    continue;
                }
                if cur_skin != Some(skinned) {
                    pass.set_pipeline(if skinned { p_skin } else { p_static });
                    pass.set_bind_group(0, bg0, &[]);
                    cur_skin = Some(skinned);
                }
                if skinned {
                    pass.set_bind_group(1, &self.bones_bg, &[(gr.palette as u64 * self.bone_stride) as u32]);
                }
                pass.set_vertex_buffer(0, mesh.vbuf.slice(..));
                pass.set_vertex_buffer(1, self.instance_buf.slice(gr.offset..gr.offset + gr.count as u64 * isize));
                pass.set_index_buffer(mesh.ibuf.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..gr.count);
            }
        };

        // shadow pass
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if shadow_on {
                draw_groups(&mut pass, &shadows, &self.p_shadow_static, &self.p_shadow_skin, &self.shadow_bg);
            }
        }

        // main pass
        {
            let (view, resolve) = match &t.msaa_color {
                Some(ms) => (ms, Some(&t.hdr)),
                None => (&t.hdr, None),
            };
            let cc = p.clear_color;
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: resolve,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: cc.x as f64,
                            g: cc.y as f64,
                            b: cc.z as f64,
                            a: 1.0,
                        }),
                        store: if resolve.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if p.draw_sky {
                pass.set_pipeline(&self.p_sky);
                pass.set_bind_group(0, &self.main_bg, &[]);
                pass.draw(0..3, 0..1);
            }
            draw_groups(&mut pass, &opaque, &self.p_static, &self.p_skin, &self.main_bg);
            draw_groups(&mut pass, &transparent, &self.p_transparent, &self.p_transparent_skin, &self.main_bg);
            if !additive.is_empty() {
                draw_groups(&mut pass, &additive, &self.p_additive, &self.p_transparent_skin, &self.main_bg);
            }
        }

        let post_pass = |enc: &mut wgpu::CommandEncoder,
                         target: &wgpu::TextureView,
                         pipeline: &wgpu::RenderPipeline,
                         bg: &wgpu::BindGroup,
                         load: bool| {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if load {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..3, 0..1);
        };

        // bloom down
        for i in 0..BLOOM_LEVELS {
            let pl = if i == 0 { &self.p_down_first } else { &self.p_down };
            post_pass(&mut enc, &t.bloom[i], pl, &t.bloom_down_bg[i], false);
        }
        // bloom up (accumulate)
        for i in (0..BLOOM_LEVELS - 1).rev() {
            post_pass(&mut enc, &t.bloom[i], &self.p_up, &t.bloom_up_bg[i], true);
        }
        // glass chain (blurred scene for frosted UI panels)
        for i in 0..3 {
            post_pass(&mut enc, &t.glass[i], &self.p_down, &t.glass_bg[i], false);
        }
        // composite
        post_pass(&mut enc, target, &self.p_composite, &t.composite_bg, false);

        // UI
        if !ui.cmds.is_empty() {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_bind_group(0, &t.ui_bg, &[]);
            pass.set_vertex_buffer(0, self.ui_vbuf.slice(..));
            pass.set_index_buffer(self.ui_ibuf.slice(..), wgpu::IndexFormat::Uint32);
            for cmd in &ui.cmds {
                if cmd.count == 0 {
                    continue;
                }
                match cmd.clip {
                    Some(c) => {
                        let x0 = c[0].max(0.0).min(width as f32) as u32;
                        let y0 = c[1].max(0.0).min(height as f32) as u32;
                        let x1 = c[2].max(0.0).min(width as f32) as u32;
                        let y1 = c[3].max(0.0).min(height as f32) as u32;
                        if x1 <= x0 || y1 <= y0 {
                            continue;
                        }
                        pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
                    }
                    None => pass.set_scissor_rect(0, 0, width, height),
                }
                pass.draw_indexed(cmd.start..cmd.start + cmd.count, 0, 0..1);
            }
        }

        queue.submit(Some(enc.finish()));
    }
}
