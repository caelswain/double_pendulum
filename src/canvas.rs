use crate::{program::RenderFn, renderer::QuadInstance};

pub struct Canvas {
    instance_list: Vec<QuadInstance>,
}

impl Canvas {
    pub(crate) fn new() -> Self {
        Self { instance_list: Vec::new() }
    } 

    pub(crate) fn build_instances(&mut self) -> &[QuadInstance] {
        &self.instance_list
    }

    pub fn draw_rect(&mut self, position: [f32;2], size: [f32; 2], angle: f32, color:[f32; 4]) {
        let rect = QuadInstance::new(position, size,0.0, angle, color);
        self.instance_list.push(rect);
    }

    pub fn draw_round_rect(&mut self, position: [f32;2], size: [f32; 2], radius: f32, angle: f32, color:[f32; 4]) {
        if radius > f32::min(size[0], size[1]) / 2.0 {
            panic!("圆角半径不要比短边还长, 不是期望的动作!")
        }
        let rect = QuadInstance::new(position, size, radius, angle, color);
        self.instance_list.push(rect);
    }

    pub fn draw_cicle(&mut self, position: [f32;2], radius: f32, angle: f32, color:[f32; 4]) {
        let rect = QuadInstance::new(position, [radius * 2.0, radius * 2.0], radius, angle, color);
        self.instance_list.push(rect);
    }

    pub fn draw_line(&mut self, from_position: [f32;2], to_position: [f32;2], boldness: f32, color:[f32; 4]) {
        let px = (from_position[0] + to_position[0]) / 2.0;
        let py = (from_position[1] + to_position[1]) / 2.0;
        let dx = to_position[0] - from_position[0];
        let dy = to_position[1] - from_position[1];
        let distance = (dx.powi(2) + dy.powi(2)).sqrt();
        let angle = dy.atan2(dx);
        let rect = QuadInstance::new([px, py], [distance, boldness], 0.0, angle, color);
        self.instance_list.push(rect);
    }

    pub(crate) fn clear(&mut self) {
        self.instance_list.clear();
    }
}