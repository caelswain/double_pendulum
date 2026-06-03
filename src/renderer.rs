use std::sync::Arc;

use wgpu::{
    Backends, CommandEncoder, CurrentSurfaceTexture, Device, Instance, InstanceDescriptor,
    PipelineCompilationOptions, Queue, RequestAdapterOptions, RenderPipeline,
    ShaderModuleDescriptor, ShaderSource,
};
use winit::window::Window;

pub(crate) struct Renderer {
    wgpu: WgpuContext,
    pipeline: RenderPipeline,
}

impl Renderer {
    pub(crate) fn new(window: Arc<Window>) -> Self {
        let wgpu = WgpuContext::new(window);
        let pipeline = create_blue_square_pipeline(&wgpu.device, &wgpu.config);
        Self {
            wgpu,
            pipeline,
        }
    }

    pub(crate) fn render<F>(&mut self, render_fn: F)
    where
        F: FnOnce(&mut CommandEncoder),
    {
        let ws = &self.wgpu;

        // 1. 获取下一帧纹理
        let current_surface_texture: CurrentSurfaceTexture = ws.surface.get_current_texture();
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
        let mut encoder = ws
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main Render Encoder"),
            });

        // 3：开启一个 RenderPass，范围块限制 RenderPass 声明周期
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.draw(0..6, 0..1);
        }

        render_fn(&mut encoder);

        // 4：提交并申请重绘
        ws.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

struct WgpuContext {
    surface: wgpu::Surface<'static>, // 新版 API 中其实不需要 'static，但保留兼容写法
    device: Device,
    queue: Queue,
    config: wgpu::SurfaceConfiguration,
}

impl WgpuContext {
    fn new(window: Arc<Window>) -> Self {
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
}

fn create_blue_square_pipeline(
    device: &Device,
    config: &wgpu::SurfaceConfiguration,
) -> RenderPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Blue Square Shader"),
        source: ShaderSource::Wgsl(include_str!("blue_square.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Blue Square Pipeline Layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Blue Square Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
