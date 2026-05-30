use std::sync::Arc;

use wgpu::{
    Backends, CurrentSurfaceTexture, Device, Instance, InstanceDescriptor, Queue,
    RequestAdapterOptions, Surface,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

mod renderer;

struct App {
    window: Option<Arc<Window>>,
    wgpu: Option<WgpuContext>,
}

struct WgpuContext {
    surface: wgpu::Surface<'static>, // 新版 API 中其实不需要 'static，但保留兼容写法
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("dp"))
                .unwrap(),
        );

        window.set_visible(true);

        self.wgpu = Some(init(window.clone()));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            // 优化 1：响应窗口大小调整
            // WindowEvent::Resized(physical_size) => {
            //     if physical_size.width > 0 && physical_size.height > 0 {
            //         self.resize(physical_size); // 内部调用 surface.configure
            //     }
            // }
            WindowEvent::RedrawRequested => {
                let ws = if let Some(s) = &self.wgpu {
                    s
                } else {
                    return;
                };

                // 1. 获取下一帧纹理
                let current_surface_texture: CurrentSurfaceTexture =
                    ws.surface.get_current_texture();
                let surface_texture = match current_surface_texture {
                    CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
                    CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
                    _ => {
                        panic!("surface_texture获取失败")
                    }
                };

                // 2. 从 SurfaceTexture 中创建 TextureView，并创建指令录制器
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    ws.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Main Render Encoder"),
                        });

                // 3：开启一个 RenderPass，范围块限制 RenderPass 声明周期
                {
                    let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Clear Screen Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.9,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                }

                // 4：提交并申请重绘
                ws.queue.submit(std::iter::once(encoder.finish()));
                surface_texture.present();

                // 持续请求重绘（对于游戏/动画应用）
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn init(window: Arc<Window>) -> WgpuContext {
    let instance_descriptor = InstanceDescriptor {
        backends: Backends::all(),
        ..InstanceDescriptor::new_without_display_handle()
    };
    let instance = Instance::new(instance_descriptor);
    let surface = instance.create_surface(window.clone()).unwrap();
    let adapter = instance.request_adapter(&RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    });
    let adapter = pollster::block_on(adapter).unwrap();
    let dq_fut = adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::empty(),
        // WebGL 后端并不支持 wgpu 的所有功能，
        // // 所以如果要以 web 为构建目标，就必须禁用一些功能。
        required_limits: if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        },
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        label: None,
        memory_hints: wgpu::MemoryHints::Performance,
        // 追踪 API 调用路径
        trace: wgpu::Trace::Off,
    });
    let (device, queue) = pollster::block_on(dq_fut).unwrap();
    let size = window.inner_size();
    let caps = surface.get_capabilities(&adapter);
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        // 通常是 Bgra8Unorm 或 Rgba8Unorm
        format: caps.formats[0],
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo, // 开启垂直同步
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);
    WgpuContext {
        surface,
        device,
        queue,
        config: surface_config,
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App {
        window: None,
        wgpu: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
