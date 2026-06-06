use std::sync::Arc;

use crate::{canvas::Canvas, renderer::{QuadInstance, Renderer}};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};

pub type UpdateFn<AppState> = Box<dyn FnMut(&mut AppState)>;
pub type RenderFn<AppState> = Box<dyn FnMut(&AppState, &mut Canvas)>;

pub struct Program<AppState> {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    window_attributes: WindowAttributes,
    canvas: Canvas,
    app_state: AppState,
    update_fn: UpdateFn<AppState>,
    render_fn: RenderFn<AppState>,
}

impl<AppState> ApplicationHandler for Program<AppState> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes.clone())
                .unwrap(),
        );

        self.window = Some(window.clone());
        self.renderer = Some(Renderer::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
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
                (self.update_fn)(&mut self.app_state);
                self.render();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        (self.update_fn)(&mut self.app_state);
        self.window.as_ref().unwrap().request_redraw();
    }
}

impl<AppState> Program<AppState> {
    pub fn new(app_state: AppState, update_fn: UpdateFn<AppState>, render_fn: RenderFn<AppState>) -> Self {
        Self {
            window: None,
            renderer: None,
            window_attributes: WindowAttributes::default(),
            canvas: Canvas::new(),
            app_state,
            update_fn,
            render_fn,
        }
    }

    fn render(&mut self) {
        self.canvas.clear();
        (self.render_fn)(&self.app_state, &mut self.canvas);
        let instances = self.canvas.build_instances();
        self.renderer.as_mut().unwrap().render(instances);
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.renderer.as_mut().unwrap().resize(size);
    }

    pub fn run(mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(&mut self).unwrap();
    }

    pub fn with_title<T: Into<String>>(mut self, title: T) -> Self {
        self.window_attributes = self.window_attributes.with_title(title);
        self
    }

    pub fn with_inner_size<S: Into<Size>>(mut self, size: S) -> Self {
        self.window_attributes = self.window_attributes.with_inner_size(size);
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.window_attributes = self.window_attributes.with_resizable(resizable);
        self
    }
}
