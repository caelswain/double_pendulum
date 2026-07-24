<!-- omit from toc -->
# 双摆（Double Pendulum）— wgpu 学习笔记

一个用 Rust + wgpu 渲染的双摆物理模拟。本文档记录了项目中 wgpu 初始化与渲染管线搭建的核心流程，便于日后回顾 GPU 编程的关键概念和设计选择。

<!-- omit from toc -->
## 目录

- [项目结构](#项目结构)
- [WgpuContext::new() — wgpu 初始化流程](#wgpucontextnew--wgpu-初始化流程)
  - [1. Instance：图形 API 入口](#1-instance图形-api-入口)
  - [2. Surface：桥接窗口系统](#2-surface桥接窗口系统)
  - [3. Adapter：选择物理 GPU](#3-adapter选择物理-gpu)
  - [4. Device \& Queue：逻辑设备与指令队列](#4-device--queue逻辑设备与指令队列)
  - [5. Surface capabilities 与配置](#5-surface-capabilities-与配置)
  - [6. surface.configure()：交换链就绪](#6-surfaceconfigure交换链就绪)
- [create\_pipeline() — 渲染管线搭建](#create_pipeline--渲染管线搭建)
  - [1. Shader Module：编译 WGSL](#1-shader-module编译-wgsl)
  - [2. Bind Group Layout：声明数据接口](#2-bind-group-layout声明数据接口)
  - [3. Uniform Buffer \& Bind Group：填充数据](#3-uniform-buffer--bind-group填充数据)
  - [4. Pipeline Layout：绑定组挂载](#4-pipeline-layout绑定组挂载)
  - [5. Render Pipeline：GPU 状态机](#5-render-pipelinegpu-状态机)
  - [6. Instance Buffer：实例化渲染](#6-instance-buffer实例化渲染)
- [关键设计决策汇总](#关键设计决策汇总)

## 项目结构

```
src/
├── main.rs        # 双摆物理（RK4 积分）、App 状态、render 回调
├── lib.rs         # 模块声明
├── program.rs     # winit 事件循环 + Program 框架
├── canvas.rs      # 绘制 API（draw_line / draw_cicle / draw_rect）
├── renderer.rs    # wgpu 初始化 + 渲染管线 + 实例化渲染
└── shader.wgsl    # WGSL 着色器（SDF 圆角矩形）
```

`renderer.rs` 是 wgpu 的核心所在，包含两个关键函数：`WgpuContext::new()` 负责 GPU 初始化，`create_pipeline()` 负责渲染管线搭建。

## WgpuContext::new() — wgpu 初始化流程

```rust
struct WgpuContext {
    surface: wgpu::Surface<'static>,
    device:  Device,
    queue:   Queue,
    config:  wgpu::SurfaceConfiguration,
}
```

四个字段恰好对应 wgpu 编程中四个最基础的概念：**画布、逻辑设备、指令队列、交换链参数**。整个 `new()` 方法就是按依赖顺序依次创建它们。

### 1. Instance：图形 API 入口

```rust
let instance = Instance::new(InstanceDescriptor {
    backends: Backends::all(),
    ..InstanceDescriptor::new_without_display_handle()
});
```

- Instance 是 wgpu 的全局上下文，代表与底层图形 API（Vulkan / Metal / DX12 / OpenGL）的连接。
- `Backends::all()` 让 wgpu 按优先级自动选择当前平台可用的最佳后端（Vulkan > Metal > DX12 > OpenGL），无需手动绑定到特定 API。跨平台时这种写法最省心。
- `new_without_display_handle()` 表示不绑定到已有窗口句柄，因为后面要通过 winit 创建 surface。

### 2. Surface：桥接窗口系统

```rust
let surface = instance.create_surface(window.clone()).unwrap();
```

- Surface 是 GPU 和窗口系统之间的"画布"。wgpu 渲染的所有内容最终都要提交到 surface，再由窗口系统呈现到屏幕。
- `create_surface` 接收 winit 的 `Arc<Window>`。winit 已经在内部处理好了平台相关的窗口句柄（Windows 的 HWND、Linux 的 X11/Wayland、macOS 的 NSView），wgpu 只需要这个抽象即可。

### 3. Adapter：选择物理 GPU

```rust
let adapter = instance.request_adapter(&RequestAdapterOptions {
    power_preference: PowerPreference::default(),
    compatible_surface: Some(&surface),
    force_fallback_adapter: false,
});
let adapter = pollster::block_on(adapter).unwrap();
```

- Adapter 是物理 GPU 的句柄。一个机器可能有多个 GPU（独显 + 核显），`request_adapter` 让 wgpu 挑选最合适的一个。
- **`compatible_surface: Some(&surface)`** 是关键参数：它确保选中的 GPU 能够真正渲染到之前创建的 surface。在双显卡笔记本上，如果 surface 创建在核显控制的窗口上却选了独显的 adapter，渲染结果将无法呈现。
- **`force_fallback_adapter: false`** 禁用软件渲染适配器（如 WARP、llvmpipe），保证使用真正的 GPU。
- **`pollster::block_on`**：`request_adapter` 返回的是 `Future`。桌面端不需要引入完整的 async runtime（如 tokio），pollster 是最轻量的"阻塞等待一个 future"方案。它的实现就是一个简单的自旋轮询器，适配 wgpu 这类紧耦合的图形 API 调用。

### 4. Device & Queue：逻辑设备与指令队列

```rust
let (device, queue) = pollster::block_on(
    adapter.request_device(&DeviceDescriptor {
        required_features: Features::empty(),
        required_limits: if cfg!(target_arch = "wasm32") {
            Limits::downlevel_webgl2_defaults()
        } else {
            Limits::default()
        },
        memory_hints: MemoryHints::Performance,
        ..
    })
).unwrap();
```

- **Device**：逻辑设备，是创建所有 GPU 资源（buffer、texture、pipeline、bind group）的工厂。它不直接对应物理 GPU，而是 adapter 提供的一个受控接口。一个 adapter 可以创建多个 device，但通常一个就够。
- **Queue**：指令提交队列。所有 `write_buffer`、`write_texture` 和 render/compute pass 的 `submit` 都通过 queue 发给 GPU。queue 是线程安全的——这是 wgpu 相比原始 Vulkan 的一个重大易用性改进：不用手动管理多个队列族和时间线信号量。
- **wasm32 条件分支**：WebGL 后端的能力集比原生 Vulkan/Metal/DX12 小得多，必须主动降级 limits，否则 `request_device` 会在浏览器中失败。
- **`MemoryHints::Performance`**：告诉驱动"要性能，不要省电"。在移动设备上这会切换到高性能 GPU；在桌面上通常没有实际影响，但它明确表达了意图。

### 5. Surface capabilities 与配置

```rust
let caps = surface.get_capabilities(&adapter);
let surface_config = SurfaceConfiguration {
    usage: TextureUsages::RENDER_ATTACHMENT,
    format: caps.formats[0],
    width: size.width,
    height: size.height,
    present_mode: PresentMode::Fifo,
    alpha_mode: caps.alpha_modes[0],
    view_formats: vec![],
    desired_maximum_frame_latency: 2,
};
```

- `surface.get_capabilities(&adapter)` 查询当前 surface + adapter 组合支持的格式和呈现模式。不同平台/GPU 支持的能力不同，动态查询比硬编码更可靠。
- **`format: caps.formats[0]`**：取列表中第一个格式。在绝大多数桌面平台上这是 `Bgra8Unorm`（8 位 BGRA），但不要假设。在移动端或 Web 上可能是 `Rgba8Unorm`。shader 中 `@location(0) vec4<f32>` 的颜色输出由 wgpu 自动转换为 surface 格式。
- **`PresentMode::Fifo`**：垂直同步模式（V-Sync）。GPU 维护一个 2-3 帧的 FIFO 缓冲区，显示器刷新时从中取一帧显示。为什么选它而不是 `Immediate` 或 `Mailbox`？
  - `Immediate`（不等待）：帧渲染完直接推到屏幕，可能导致画面撕裂（上半帧是旧内容，下半帧是新内容）。适合延迟敏感、画面正确性不关键的场景（如部分 FPS 游戏）。
  - `Mailbox`（单缓冲替换）：只保留最新一帧，旧帧直接丢弃。延迟比 Fifo 低，但若渲染速率远高于刷新率会浪费 GPU 算力。
  - **Fifo**：渲染帧若快于刷新率会阻塞等待 vsync，达到帧率与刷新率同步。对于这个双摆项目，画面变化平滑、没有延迟要求，Fifo 是最自然的选择——无撕裂、不浪费算力。
- **`desired_maximum_frame_latency: 2`**：允许 GPU 提前渲染最多 2 帧。降低延迟可以设 1，但可能增加卡顿风险；提高可以更平滑但增加输入延迟。2 是多数桌面平台的默认值，兼顾流畅与响应。

### 6. surface.configure()：交换链就绪

```rust
surface.configure(&device, &surface_config);
```

将配置写入 surface，内部创建交换链（swap chain）。此步必须在 device 创建之后，而且每次窗口 resize 后都需要重新调用。这也是为什么 `WgpuContext` 要保有 `config` 字段——resize 时修改宽高后直接重配，无需重建其他资源。

至此，`WgpuContext` 初始化完成。总结依赖链：

```
Instance → Surface → Adapter → (Device, Queue) → Surface.configure()
```

每一步都严格依赖前一步的输出，这是 wgpu 强制保证的状态机顺序。

## create_pipeline() — 渲染管线搭建

`create_pipeline` 构建了从 WGSL 源码到 GPU 可执行渲染管线的整个链条。

### 1. Shader Module：编译 WGSL

```rust
let shader = device.create_shader_module(ShaderModuleDescriptor {
    source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    ..
});
```

- `include_str!` 在编译期将 `shader.wgsl` 内容嵌入二进制，运行时零 I/O。对于单个小 shader 这是最省事的方案；大型项目可能需要独立的 shader 编译流程（如 naga 预编译 SPIR-V），但学习阶段没必要。
- WGSL 是 wgpu 的原生着色语言。使用 WGSL 而非 GLSL/SPIR-V 可以避免引入额外的编译工具链（shaderc / naga-cli），直接由 wgpu 内置的 naga 编译器处理。

### 2. Bind Group Layout：声明数据接口

```rust
let view_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    entries: &[BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::VERTEX_FRAGMENT,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        ..
    }],
});
```

- Bind Group Layout（BGL）是 CPU 侧数据和 Shader 之间的"类型契约"。它声明：shader 的 `@group(0) @binding(0)` 会收到一个 uniform buffer。
- `visibility: VERTEX_FRAGMENT` 表示这个 buffer 在顶点和片元阶段都可见——虽然 fragment shader 不需要它，但声明为两者可见不影响正确性，只是让 GPU 驱动少做一点优化。如果只声明 `VERTEX`，驱动可以更激进地排布寄存器，但对于这样简单的 shader 差别可以忽略。
- **为什么需要一个 BGL？**：wgpu 是显式 API，shader 不会自动"看到"某个 buffer。BGL + Bind Group 的双层设计把"接口声明"和"实例绑定"分开，使得同一个 shader 可以配合不同的数据实例（例如双缓冲、多 pass 切换）。
- `has_dynamic_offset: false`：buffer 绑定位置固定不变。如果需要一个 buffer 的不同偏移量用于不同 draw call，可以设 `true`，用 `set_bind_group(0, &bg, &[offset])` 动态指定。此处不需要。

### 3. Uniform Buffer & Bind Group：填充数据

```rust
let view_ubo = device.create_buffer_init(&BufferInitDescriptor {
    contents: bytemuck::bytes_of(&view_uniform),
    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
});

let view_bg = device.create_bind_group(&BindGroupDescriptor {
    layout: &view_bgl,
    entries: &[BindGroupEntry {
        binding: 0,
        resource: view_ubo.as_entire_binding(),
    }],
});
```

- `create_buffer_init` 是 wgpu 的便利方法：创建 buffer 的同时填入初始数据（内部分配 staging buffer 并拷贝）。比手动 `create_buffer` + `write_buffer` 省事。
- `usage: UNIFORM | COPY_DST`：`UNIFORM` 让它可以绑定到 uniform 槽位；`COPY_DST` 让后续 `write_buffer`（resize 时更新分辨率）能写入新数据。只设 `UNIFORM` 的话 buffer 内容在创建后就不可变。
- Bind Group 是 BGL 的具体实例：把一个真实的 buffer 绑定到 `binding=0`。`as_entire_binding()` 绑定整个 buffer，等价于"偏移 0，长度 = buffer 全长"。

### 4. Pipeline Layout：绑定组挂载

```rust
let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
    bind_group_layouts: &[Some(&view_bgl)],
    ..
});
```

把 BGL 数组挂载到 pipeline。shader 中 `@group(0)` 对应数组的第 0 号元素。一个 pipeline 可以有多个 bind group（例如 group 0 是每帧不变的 uniform，group 1 是每物体变化的纹理），此处只需要一个。

### 5. Render Pipeline：GPU 状态机

```rust
let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
    vertex: VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[QuadInstance::layout()],
    },
    fragment: Some(FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(ColorTargetState {
            format: config.format,
            blend: Some(BlendState::REPLACE),
            write_mask: ColorWrites::ALL,
        })],
    }),
    primitive: PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        front_face: FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: PolygonMode::Fill,
        ..
    },
    ..
});
```

**Vertex State**

- `buffers: &[QuadInstance::layout()]` 声明了顶点缓冲的布局——但注意这个 buffer 是**实例化**的（`step_mode: VertexStepMode::Instance`），每 6 个顶点（两个三角形组成的单位正方形）的顶点索引由 `@builtin(vertex_index)` 自动循环，而 position / size / color 来自每个 instance 的数据。这就是为什么不需要显式的顶点缓冲：6 个顶点坐标硬编码在 WGSL 的 `POSITIONS` 数组里。

**Fragment State**

- `blend: Some(BlendState::REPLACE)`：片元输出直接覆盖 framebuffer 中的已有颜色，不做 alpha 混合。如果需要半透明效果，这里要改成 `ALPHA_BLENDING`，但双摆画面只有不透明的杆和球，REPLACE 更简单高效。
- `format: config.format` 必须与 surface 的格式一致，否则 pipeline 创建失败（wgpu 会校验）。

**Primitive State**

- `topology: TriangleList`：每 3 个顶点构成一个三角形。这是 GPU 的原生图元，其他拓扑（LineList、PointList 等）最终也会被驱动展开。
- `front_face: Ccw`：逆时针（Counter-Clockwise）顶点顺序的面视为正面。这与 `POSITIONS` 数组的缠绕顺序一致。
- **`cull_mode: None`**：不剔除背面。一般 3D 渲染会剔除背面（`Back`）以节省片元计算，但这里 Quad 会旋转——旋转 180° 时"正面"变成背面，剔除就会让图形消失。设为 `None` 虽然多做了一倍的片元计算，但换来了正确性。

### 6. Instance Buffer：实例化渲染

```rust
let instance_buffer = device.create_buffer(&BufferDescriptor {
    size: 1000 * size_of::<QuadInstance>() as u64,
    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
    mapped_at_creation: false,
});
```

这是整个渲染架构的关键设计：

- **预分配固定大小**（1000 个 QuadInstance）而非每帧动态扩容。GPU 分配 buffer 比较昂贵（需要驱动层的内存映射），一次性分配然后反复 `write_buffer` 覆盖内容可以完全消除每帧的分配开销。
- `usage: VERTEX | COPY_DST`：既是顶点数据源（GPU 从它读），又是 CPU 写入目标（`write_buffer` 写它）。
- **实例化渲染的工作流**：
  1. 主循环中 `canvas.build_instances()` 收集所有绘制命令到一个 `Vec<QuadInstance>`
  2. `queue.write_buffer(&self.instances_buffer, 0, bytemuck::cast_slice(instances))` 把实例数据上传到 GPU
  3. `render_pass.draw(0..6, 0..instances.len() as u32)` 绘制 6 个顶点（一个 quad），并**重复 instances.len() 次**，每次自动递增 `@location(0..4)` 的属性值

这种方式只需要一次 draw call 就能渲染所有图形（杆、球、背景矩形），而不需要为每个图形单独调用 `draw()`。Draw call 数量是 GPU 渲染的常见瓶颈，实例化是减少 draw call 的标准做法。

## 关键设计决策汇总

| 决策 | 做法 | 理由 |
|------|------|------|
| 异步处理 | `pollster::block_on` | 桌面端不需要完整 async runtime，轻量轮询器足够 |
| 后端选择 | `Backends::all()` | 自动适配平台最优后端，消除平台相关代码分支 |
| 垂直同步 | `PresentMode::Fifo` | 双摆画面变化平滑，不需要极限低延迟；Fifo 避免撕裂且不浪费算力 |
| 分辨率传递 | Uniform buffer（非硬编码） | resize 时只需 `write_buffer` 更新内容，无需重建 pipeline |
| 图形绘制 | 实例化渲染（6 顶点 + N 实例） | 一次 draw call 绘制所有图形，消除每物体一个 draw call 的开销 |
| Instance buffer | 固定大小预分配 | 避免每帧 GPU 内存分配，`write_buffer` 覆盖远快于 `create_buffer` |
| 着色语言 | WGSL | 无需外部编译工具链，由 wgpu 内置 naga 直接编译 |
| Shader 嵌入 | `include_str!` | 编译期嵌入，零运行时 I/O |
| 背面剔除 | `cull_mode: None` | 旋转后的 Quad 可能暴露背面，不剔除保证正确性 |
| 颜色混合 | `BlendState::REPLACE` | 不需要半透明效果，直接覆盖最高效 |
| 基础几何 | 硬编码 6 顶点（单位正方形） | 所有图形（线、圆、矩形）都是同一个 Quad 的变换，shader 中 SDF 根据 instance 参数区分形状 |
| 多后端兼容 | wasm32 降级 limits | 同一份代码可编译到 Web（WebGL），不需要条件编译两套逻辑 |
