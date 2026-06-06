use std::f32::{self, consts::PI};

use std::time::Instant;

use double_pendulum::program::*;
use double_pendulum::canvas::Canvas;
use winit::dpi::{PhysicalSize, Size};

struct App {
    l1: f32,
    l2: f32,
    m1: f32,
    m2: f32,
    theta1: f32,
    theta2: f32,
    omega1: f32,
    omega2: f32,
    g: f32,
    tick: Instant,
}

#[derive(Clone, Copy)]
struct PendulumState {
    theta1: f32,
    theta2: f32,
    omega1: f32,
    omega2: f32,
}

fn derivatives(app: &mut App, state: PendulumState) -> PendulumState {
    // 时间步长
    let dt: f32 = app.tick.elapsed().as_secs_f32();
    app.tick = Instant::now();

    let l1 = app.l1;
    let l2 = app.l2;
    let m1 = app.m1;
    let m2 = app.m2;
    let g = app.g;
    let (theta1, theta2) = (state.theta1, state.theta2);
    let (omega1, omega2) = (state.omega1, state.omega2);

    let delta = theta1 - theta2;
    let sin_delta = delta.sin();
    let cos_delta = delta.cos();

    let denom = l1 * (m1 + m2 * sin_delta * sin_delta);
    let term1 = m2 * cos_delta * (l1 * omega1 * omega1 * sin_delta - g * theta2.sin());
    let term2 = m2 * l2 * omega2 * omega2 * sin_delta;
    let term3 = (m1 + m2) * g * theta1.sin();
    let alpha1 = (-term1 - term2 - term3) / denom;

    let alpha2 = (-l1 * alpha1 * cos_delta + l1 * omega1 * omega1 * sin_delta - g * theta2.sin()) / l2;

    PendulumState {
        theta1: omega1,
        theta2: omega2,
        omega1: alpha1,
        omega2: alpha2,
    }
}

fn add_scaled(state: PendulumState, delta: PendulumState, scale: f32) -> PendulumState {
    PendulumState {
        theta1: state.theta1 + delta.theta1 * scale,
        theta2: state.theta2 + delta.theta2 * scale,
        omega1: state.omega1 + delta.omega1 * scale,
        omega2: state.omega2 + delta.omega2 * scale,
    }
}

fn update(app: &mut App) {
    let dt: f32 = app.tick.elapsed().as_secs_f32();
    app.tick = Instant::now();

    let state = PendulumState {
        theta1: app.theta1,
        theta2: app.theta2,
        omega1: app.omega1,
        omega2: app.omega2,
    };

    let k1 = derivatives(app, state);
    let k2 = derivatives(app, add_scaled(state, k1, dt * 0.5));
    let k3 = derivatives(app, add_scaled(state, k2, dt * 0.5));
    let k4 = derivatives(app, add_scaled(state, k3, dt));

    let next = PendulumState {
        theta1: state.theta1 + dt * (k1.theta1 + 2.0 * k2.theta1 + 2.0 * k3.theta1 + k4.theta1) / 6.0,
        theta2: state.theta2 + dt * (k1.theta2 + 2.0 * k2.theta2 + 2.0 * k3.theta2 + k4.theta2) / 6.0,
        omega1: state.omega1 + dt * (k1.omega1 + 2.0 * k2.omega1 + 2.0 * k3.omega1 + k4.omega1) / 6.0,
        omega2: state.omega2 + dt * (k1.omega2 + 2.0 * k2.omega2 + 2.0 * k3.omega2 + k4.omega2) / 6.0,
    };

    app.theta1 = next.theta1.rem_euclid(2.0 * PI);
    app.theta2 = next.theta2.rem_euclid(2.0 * PI);
    app.omega1 = next.omega1;
    app.omega2 = next.omega2;
}

fn render(app: &App, canvas: &mut Canvas) {
    let x1 = app.l1 * f32::sin(app.theta1);
    let x2 = x1 + app.l2 * f32::sin(app.theta2);
    let y1 = app.l1 * f32::cos(app.theta1);
    let y2 = y1 + app.l2 * f32::cos(app.theta2);
    canvas.draw_line([450.0, 300.0], [450.0 + x1, 300.0-y1], 3.0, [0.35, 0.75, 0.65, 1.0]);
    canvas.draw_line([450.0 + x1, 300.0-y1], [450.0 + x2, 300.0-y2], 3.0, [0.35, 0.75, 0.65, 1.0]);
    canvas.draw_cicle([450.0 + x1, 300.0-y1], 10.0, 0.0, [0.15, 0.55, 0.7, 1.0]);
    canvas.draw_cicle([450.0 + x2, 300.0-y2], 20.0, 0.0, [0.65, 0.55, 0.45, 1.0]);
}

fn main() {
    let app = App{
        l1: 140.0,
        l2: 115.0,
        m1: 18.0,
        m2: 14.0,
        theta1: PI * 0.82,
        theta2: PI * 0.57,
        omega1: 0.0,
        omega2: 0.08,
        g: 650.0,
        tick: Instant::now(),
    };
    Program::new(app, Box::new(update), Box::new(render))
        .with_inner_size(Size::Physical(PhysicalSize::new(900, 600)))
        .with_resizable(false)
        .with_title("dp")
        .run();
}
