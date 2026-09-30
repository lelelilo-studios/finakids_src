//! GPU device creation and capability detection.

use wgpu::TextureFormat;

#[derive(Clone, Debug)]
pub struct GpuInfo {
    pub backend: wgpu::Backend,
    pub name: String,
    pub is_webgl: bool,
    pub is_mobile: bool,
    pub hdr_format: TextureFormat,
    pub msaa: u32,
    pub shadow_size: u32,
    pub render_scale: f32,
}

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub info: GpuInfo,
}

impl Gpu {
    pub async fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Gpu {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: surface,
                apply_limit_buckets: false,
            })
            .await
            .expect("No se encontró un adaptador gráfico compatible");
        let ainfo = adapter.get_info();
        log::info!("GPU: {} ({:?})", ainfo.name, ainfo.backend);

        let is_webgl = ainfo.backend == wgpu::Backend::Gl && cfg!(target_arch = "wasm32");
        let is_mobile = cfg!(any(target_os = "android", target_os = "ios"));
        let limits = wgpu::Limits::downlevel_webgl2_defaults()
            .using_resolution(adapter.limits())
            .using_alignment(adapter.limits());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("finakids"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await
            .expect("No se pudo crear el dispositivo gráfico");

        // HDR target format
        let f16 = adapter.get_texture_format_features(TextureFormat::Rgba16Float);
        let hdr_ok = f16
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            && f16.flags.contains(wgpu::TextureFormatFeatureFlags::FILTERABLE)
            && f16.flags.contains(wgpu::TextureFormatFeatureFlags::BLENDABLE);
        let hdr_format = if hdr_ok {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::Rgba8Unorm
        };
        let hf = adapter.get_texture_format_features(hdr_format);
        let df = adapter.get_texture_format_features(TextureFormat::Depth32Float);
        let msaa_ok = hf.flags.sample_count_supported(4)
            && hf.flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
            && df.flags.sample_count_supported(4);
        let msaa = if msaa_ok { 4 } else { 1 };

        let low = is_webgl || is_mobile;
        let info = GpuInfo {
            backend: ainfo.backend,
            name: ainfo.name.clone(),
            is_webgl,
            is_mobile,
            hdr_format,
            msaa,
            shadow_size: if low { 1024 } else { 2048 },
            render_scale: if is_mobile { 0.8 } else { 1.0 },
        };
        log::info!("{info:?}");
        Gpu {
            instance,
            adapter,
            device,
            queue,
            info,
        }
    }
}
