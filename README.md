# double_pendulum

用 Rust + wgpu 渲染的双摆物理模拟（RK4 积分）。

> **这是一个学习项目。** 借双摆这个具体问题，把 wgpu 的初始化、渲染管线、实例化渲染和 WGSL 着色器亲手走一遍；顺带实验「自研 2D 即时渲染框架」——线、圆、圆角矩形统一抽象为 `QuadInstance`，整帧只发一次 draw call。框架层 API 设计受 [iced](https://github.com/iced-rs/iced) 启发。

## 构建与运行

```sh
cargo run --release
```

## 设计要点

- **全部图形 = 一个单位正方形 + SDF 参数**：`draw_line` 是旋转的细长 Quad，`draw_circle` 是圆角矩形的特例，`draw(0..6, 0..n)` 画完整帧
- **`Canvas` 只收集命令**：纯 CPU 侧的 `Vec<QuadInstance>`，不持有 GPU 资源；`update` 与 `render` 分离，互不知情
- **零每帧分配**：实例缓冲固定预分配、每帧 `write_buffer` 覆盖；着色器用 `include_str!` 编译期嵌入二进制
- **`draw_round_rect` 有前置条件**：圆角半径不得超过短边的一半，违反时直接 `panic`（是约束检查，不是静默降级）

## 已知问题

- **支点与尺寸无关性缺失**：`render()` 把屏幕中心写死为 `[450.0, 300.0]`，假定窗口恒为 900×600。在缩放的显示器上（例如 scale = 1.5，900 逻辑像素实际渲染为 1350×900）支点会偏向左下。正确做法是从渲染目标尺寸算中心；渲染器已经通过 `ViewUniform.resolution` 持有该尺寸，只是还没暴露给 `render` 回调。（当前窗口不可缩放，但 DPI 缩放仍会触发这个问题。）
- 时间步长夹紧在 `1/240 ~ 1/30 s`（防积分发散），更稳妥的做法是固定步长 + 累加器
- 错误处理全部按「致命」处理：初始化失败 `unwrap`、surface 获取失败直接 `panic`。对学习项目够用，但没有降级路径（比如 `get_current_texture` 拿到超时时，可以重配 surface 或跳过本帧）
