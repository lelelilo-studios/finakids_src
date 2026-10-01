//! Window / surface lifecycle and the main loop for every platform.

use crate::game::{Game, GameConfig};
use crate::gfx::{Gpu, Renderer};
use crate::input::{GameKey, Input};
use std::sync::Arc;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Fullscreen, Window, WindowId};

pub enum UserEvent {
    Ready(Box<Ready>),
}

pub struct Ready {
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
}

struct Running {
    gpu: Gpu,
    surface: Option<wgpu::Surface<'static>>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    game: Game,
}

pub struct App {
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    state: Option<Running>,
    loading: bool,
    input: Input,
    last: Instant,
    cfg: GameConfig,
    occluded: bool,
}

impl App {
    pub fn new(event_loop: &EventLoop<UserEvent>, cfg: GameConfig) -> App {
        App {
            proxy: event_loop.create_proxy(),
            window: None,
            state: None,
            loading: false,
            input: Input::default(),
            last: Instant::now(),
            cfg,
            occluded: false,
        }
    }

    fn surface_config(gpu: &Gpu, surface: &wgpu::Surface<'_>, w: u32, h: u32) -> wgpu::SurfaceConfiguration {
        let caps = surface.get_capabilities(&gpu.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb() && matches!(f, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm))
            .unwrap_or_else(|| caps.formats[0].remove_srgb_suffix());
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes[0]
        };
        let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes[0]
        };
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: w.max(1),
            height: h.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        }
    }

    fn start_init(&mut self, window: Arc<Window>, display: winit::event_loop::OwnedDisplayHandle) {
        self.loading = true;
        let proxy = self.proxy.clone();
        let fut = async move {
            #[cfg(not(target_arch = "wasm32"))]
            let mut desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display));
            #[cfg(target_arch = "wasm32")]
            let mut desc = {
                let _ = display;
                wgpu::InstanceDescriptor::new_without_display_handle()
            };
            desc.backends = wgpu::Backends::all();
            #[cfg(target_arch = "wasm32")]
            let instance = wgpu::util::new_instance_with_webgpu_detection(desc).await;
            #[cfg(not(target_arch = "wasm32"))]
            let instance = wgpu::Instance::new(desc);
            let surface = instance.create_surface(window.clone()).expect("surface");
            let gpu = Gpu::new(instance, Some(&surface)).await;
            let _ = proxy.send_event(UserEvent::Ready(Box::new(Ready { gpu, surface })));
        };
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(fut);
        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(fut);
    }

    fn redraw(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(st) = self.state.as_mut() else {
            return;
        };
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().clamp(0.0001, 0.1);
        self.last = now;

        if self.input.was_pressed(GameKey::Fullscreen) && !cfg!(any(target_os = "android", target_os = "ios")) {
            if window.fullscreen().is_some() {
                window.set_fullscreen(None);
            } else {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }

        let (w, h) = (st.config.width, st.config.height);
        st.game.update(dt, &mut self.input, w, h);
        self.input.begin_frame();

        let Some(surface) = st.surface.as_ref() else {
            return;
        };
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) => f,
            wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(&st.gpu.device, &st.config);
                window.request_redraw();
                return;
            }
            other => {
                log::warn!("surface error: {other:?}");
                window.request_redraw();
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        st.game.render(&st.gpu, &mut st.renderer, &view, w, h);
        window.pre_present_notify();
        st.gpu.queue.present(frame);
        #[cfg(target_arch = "wasm32")]
        crate::platform::hide_loading();
        if st.game.quit_requested() && !cfg!(target_arch = "wasm32") {
            std::process::exit(0);
        }
        window.request_redraw();
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(st) = self.state.as_mut() {
            st.game.set_audio_paused(false);
            // Android: recreate the surface after resume
            if st.surface.is_none() {
                if let Some(window) = &self.window {
                    let surface = st.gpu.instance.create_surface(window.clone()).expect("surface");
                    let size = window.inner_size();
                    st.config.width = size.width.max(1);
                    st.config.height = size.height.max(1);
                    surface.configure(&st.gpu.device, &st.config);
                    st.surface = Some(surface);
                    window.request_redraw();
                }
            }
            return;
        }
        if self.loading {
            return;
        }
        #[allow(unused_mut)]
        let mut attrs = Window::default_attributes()
            .with_title("Finakids")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(480.0, 320.0));
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("finakids-canvas"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            attrs = attrs.with_canvas(canvas).with_prevent_default(true).with_focusable(true);
        }
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        self.window = Some(window.clone());
        let display = event_loop.owned_display_handle();
        self.start_init(window, display);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Ready(ready) => {
                let Ready { gpu, surface } = *ready;
                let window = self.window.clone().expect("window");
                let size = window.inner_size();
                let config = Self::surface_config(&gpu, &surface, size.width, size.height);
                surface.configure(&gpu.device, &config);
                let mut renderer = Renderer::new(&gpu, config.format);
                let game = Game::new(&gpu, &mut renderer, self.cfg.clone());
                self.state = Some(Running {
                    gpu,
                    surface: Some(surface),
                    config,
                    renderer,
                    game,
                });
                self.loading = false;
                self.last = Instant::now();
                window.request_redraw();
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(st) = self.state.as_mut() {
            st.game.save();
            st.game.set_audio_paused(true);
            if cfg!(target_os = "android") {
                st.surface = None;
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        self.input.handle_event(&event);
        match event {
            WindowEvent::CloseRequested => {
                if let Some(st) = self.state.as_mut() {
                    st.game.save();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(st) = self.state.as_mut() {
                    st.config.width = size.width.max(1);
                    st.config.height = size.height.max(1);
                    if let Some(s) = &st.surface {
                        s.configure(&st.gpu.device, &st.config);
                    }
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::Occluded(o) => {
                self.occluded = o;
                if !o {
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if !self.occluded {
                    self.redraw();
                }
            }
            _ => {}
        }
    }
}

/// Builds the event loop and runs the game.
pub fn run_with(event_loop: EventLoop<UserEvent>, cfg: GameConfig) {
    let app = App::new(&event_loop, cfg);
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = app;
        event_loop.run_app(&mut app).expect("event loop");
    }
}

// ------------------------------------------------------------------ headless screenshots

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub fn screenshot(cfg: GameConfig, path: &str, width: u32, height: u32, frames: u32) {
    let fut = async move {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(desc);
        let gpu = Gpu::new(instance, None).await;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer = Renderer::new(&gpu, format);
        let mut game = Game::new(&gpu, &mut renderer, cfg);
        let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut input = Input::default();
        let t0 = Instant::now();
        for f in 0..frames {
            game.update(1.0 / 30.0, &mut input, width, height);
            input.begin_frame();
            if f + 45 >= frames {
                game.render(&gpu, &mut renderer, &view, width, height);
            }
        }
        game.render(&gpu, &mut renderer, &view, width, height);
        let bpr = (width * 4).div_ceil(256) * 256;
        let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (bpr * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bpr),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit(Some(enc.finish()));
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let data = slice.get_mapped_range().expect("map").to_vec();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let row = &data[(y * bpr) as usize..(y * bpr + width * 4) as usize];
            pixels.extend_from_slice(row);
        }
        let file = std::fs::File::create(path).expect("create png");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().expect("png header");
        w.write_image_data(&pixels).expect("png data");
        log::info!("screenshot saved to {path} ({:.2}s)", t0.elapsed().as_secs_f32());
    };
    pollster::block_on(fut);
}
