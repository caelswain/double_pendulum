use winit::{application::ApplicationHandler, window::Window};

struct Program {
    window: Window,
    renderer: Renderer,
}

impl ApplicationHandler for Program {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        todo!()
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        todo!()
    }
}
