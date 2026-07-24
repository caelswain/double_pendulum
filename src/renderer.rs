use std::sync::Arc;

use wgpu::{
    Backends, BindGroup, BindGroupLayout, CommandEncoder, CurrentSurfaceTexture, Device, Instance, InstanceDescriptor, PipelineCompilationOptions, Queue, RenderPipeline, RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, util::DeviceExt
};
use winit::{dpi::PhysicalSize, window::Window};


pub(crate) struct Renderer {
    wgpu: WgpuContext,
    pipeline: RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: BindGroup,
    instances_buffer: wgpu::Buffer,
}

impl Renderer {
    pub(crate) fn new(window: Arc<Window>) -> Self {
        let wgpu = WgpuContext::new(window);
        let (pipeline, uniform_buffer, bind_group, instances_buffer) = create_pipeline(&wgpu.device, &wgpu.config);
        Self { wgpu, pipeline, uniform_buffer, bind_group, instances_buffer }
    }

    pub(crate) fn render(&mut self, instances: &[QuadInstance]) {
        let ws = &self.wgpu;

        // 向 Surface 申请一个可以写入的 Back Buffer。
        // 本帧所有渲染最终都会写入这张纹理。
        let current_surface_texture: CurrentSurfaceTexture = ws.surface.get_current_texture();
        let surface_texture = match current_surface_texture {
            CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            _ => {
                panic!("surface_texture获取失败")
            }
        };

        // 从 SurfaceTexture 中创建 TextureView，TextureView 是 GPU 实际访问 Texture 的入口
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // 创建指令录制器
        // CommandEncoder 用于录制 GPU 指令。
        // 这里只是记录，不会立即执行。
        let mut encoder = ws
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main Render Encoder"),
            });

        // 3：开启一个 RenderPass，范围块限制 RenderPass 声明周期
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Draw Instance Pass"),
                // 告诉 GPU：
                // 这次画到哪里？
                // 通常Surface Texture
                // 以后阴影会变成：Shadow Map
                // 后处理会变成HDR：Texture
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                    // 说明前面的Framebuffer 内容怎么处理，这里选择清除
                    // Clear：清屏。
                    // Load：保留已有内容继续绘制。
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.9,
                            a: 1.0,
                        }),
                        // RenderPass 结束后是否保留绘制结果。
                        // Store：保存。
                        // Discard：结果可丢弃。
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // 指定后续 draw() 使用哪条渲染管线。
            // 包括 Shader、Vertex Layout、Blend 等状态。
            render_pass.set_pipeline(&self.pipeline);
            // 将 BindGroup 绑定到 group=0。
            // Shader 中 @group(0) 将读取这里绑定的资源。
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            // 利用GPU 数据上传通道向实例缓冲区写入数据
            ws.queue.write_buffer(&self.instances_buffer, 0, bytemuck::cast_slice(instances));
           // 指定 Vertex Buffer。
            // 后续 draw() 时 Vertex Shader 会从这里读取输入。
            render_pass.set_vertex_buffer(0, self.instances_buffer.slice(..));
            // 让管线将把前6个顶点绘制出来，GPU 会根据管线的描述去组装图元并执行顶点着色器和片段着色器
            // 每个实例绘制 6 个 Vertex（两个三角形组成一个 Quad）。
            // 第二个参数表示实例数量。
            render_pass.draw(0..6, 0..(instances.len() as u32));
        }

        // 将录制好的 GPU 指令提交给 GPU 执行。
        ws.queue.submit(std::iter::once(encoder.finish()));
        // 交换前后台缓冲，将本帧显示到屏幕。
        surface_texture.present();
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let wgpu = &mut self.wgpu;
        wgpu.config.width = size.width;
        wgpu.config.height = size.height;
        wgpu.surface.configure(&wgpu.device, &wgpu.config);
        let uniform = ViewUniform::new(size.width as f32, size.height as f32);
        wgpu.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
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

fn create_pipeline(
    device: &Device,
    config: &wgpu::SurfaceConfiguration,
) -> (RenderPipeline, wgpu::Buffer, BindGroup, wgpu::Buffer) {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Sdf Shader"),
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    // 声明这一组资源（Group）的接口。
    // GPU 会根据这里的声明检查 Shader 中 @group/@binding 是否匹配。
    let view_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("View BGL"),
        // 声明 binding=0 的资源类型。
        // 必须与 WGSL 中
        // @group(0) @binding(0)
        // 的声明保持一致。
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            // 指定哪些 Shader Stage 可以访问。
            // 这里只允许 Vertex 和 Fragment Shader 使用。
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // CPU 端维护的 Uniform 数据。
    // 后续会复制到 GPU Buffer。
    let view_uniform = ViewUniform {
        resolution: [config.width as f32, config.height as f32],
        _pad: [0.0, 0.0],
    };

    // 创建 GPU Uniform Buffer，并把刚刚的 CPU 数据上传进去。
    let view_ubo = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("View UBO"),
        contents: bytemuck::bytes_of(&view_uniform),
        // 指定 Buffer 的用途。
        // UNIFORM：Shader 可以读取。
        // COPY_DST：CPU 后续可以继续写入。
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // 创建 BindGroup。
    // 将 GPU Buffer 绑定到 binding=0。
    let view_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("View BG"),
        layout: &view_bgl,
        // 指定 binding=0 对应哪个 GPU Buffer。
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: view_ubo.as_entire_binding(),
        }],
    });

    // 声明 Pipeline 使用哪些 BindGroup。
    // 顺序必须与 Shader 中的 @group(...) 一致。
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Sdf Pipeline Layout"),
        bind_group_layouts: &[Some(&view_bgl)],
        immediate_size: 0,
    });

    // 创建 Render Pipeline。
    // 描述 GPU 如何完成一次绘制。
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Sdf Pipeline"),
        layout: Some(&pipeline_layout),
        // 指定顶点着色器及其入口点和编译选项
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            // Vertex Buffer 的解析规则。
            // GPU 会按照这里解析 QuadInstance。
            buffers: &[QuadInstance::layout()],
        },
        // 指定片段着色器及其入口点和编译选项
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            // 最终颜色输出到哪个颜色附件。
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                // blend 模式，REPLACE 表示直接覆盖，ALPHA_BLENDING 表示使用 alpha 混合等
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            // 说明顶点列表如何被解释为图元，TriangleList 表示每三个顶点组成一个三角形，无重叠
            // draw(6)
            // 会解释为
            //
            // (0,1,2)
            //
            // (3,4,5)
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
    });

    // GPU Vertex Buffer。
    // 每个实例对应一个 QuadInstance。
    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Instance Buffer"),
        // 预留空间，存储 1000 个实例的数据
        size: 1000 * size_of::<QuadInstance>() as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    (
        pipeline,
        view_ubo,
        view_bg,
        instance_buffer,
    )
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
    resolution: [f32; 2], // 屏幕像素尺寸，例如 (width, height)
    _pad: [f32; 2],
}

impl ViewUniform {
    fn new(width: f32, height: f32) -> Self {
        Self { resolution: [width, height], _pad: [0.0, 0.0] }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pos_px: [f32; 2],  // 中心
    size_px: [f32; 2], // 宽高（像素）
    radius_px: f32,    // 圆角半径（像素）
    angle: f32,
    color: [f32; 4],
}

impl QuadInstance {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadInstance>() as u64,

            step_mode: wgpu::VertexStepMode::Instance,

            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },

                // size
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },

                // radius
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },

                // angle
                wgpu::VertexAttribute {
                    offset: 20,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },

                // color
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    pub fn new(position: [f32; 2],  size: [f32; 2], radius: f32, angle: f32, color: [f32; 4]) -> Self {
        Self {
            pos_px: position,  // 中心
            size_px: size, // 宽高（像素）
            radius_px: radius,    // 圆角半径（像素）
            angle,
            color,
        }
    }
}