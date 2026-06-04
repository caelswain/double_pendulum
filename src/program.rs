use std::sync::Arc;

use crate::renderer::Renderer;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

pub struct Program {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
}

impl ApplicationHandler for Program {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("dp"))
                .unwrap(),
        );

        self.window = Some(window.clone());
        self.renderer = Some(Renderer::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            // 优化 1：响应窗口大小调整
            WindowEvent::Resized(physical_size) => {
                if physical_size.width > 0 && physical_size.height > 0 {
                    self.resize(physical_size); // 内部调用 surface.configure
                }
            }
            WindowEvent::RedrawRequested => {
                self.renderer.as_mut().unwrap().render(|renderer| {});
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

impl Program {
    pub fn new() -> Self {
        Self {
            window: None,
            renderer: None,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.renderer.as_mut().unwrap().resize(size);
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(&mut self).unwrap();
    }
}
