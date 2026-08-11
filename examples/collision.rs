use std::{
    num::NonZero,
    rc::Rc,
    time::{Duration, Instant},
};

use softbuffer::{Buffer, Context, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{MouseButton, StartCause, WindowEvent},
    event_loop::{self, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowAttributes},
};

use mini_ecs::{entity::Entity, world::World};

#[derive(Debug, Default, Clone, Copy)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Position {
    pub ox: f32,
    pub oy: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Color {
    pub r: u32,
    pub g: u32,
    pub b: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Radius {
    pub r: f32,
}

//winit
#[derive(Debug)]
struct App {
    context: Context<OwnedDisplayHandle>,
    state: AppState,
    world: World,
    cursor_position: Option<PhysicalPosition<f64>>,
    last_frame_time: Instant,
    last_mouse_input_time: Instant,
}

#[derive(Debug)]
enum AppState {
    Initial,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    },
}

impl ApplicationHandler for App {
    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let StartCause::Init = cause {
            let window_attr = WindowAttributes::default();
            let window = event_loop
                .create_window(window_attr)
                .expect("Failed to creating window");
            self.state = AppState::Suspended {
                window: Rc::new(window),
            };
        }
    }
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let AppState::Suspended { window } = &mut self.state else {
            unreachable!("got resumed event while not suspended");
        };

        let mut surface =
            Surface::new(&self.context, window.clone()).expect("Failed to creating surface");

        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZero::new(size.width), NonZero::new(size.height)) {
            let _ = surface.resize(width, height);
        }

        self.state = AppState::Running { surface };
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got resumed event while not running");
        };

        let window = surface.window().clone();
        self.state = AppState::Suspended { window }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let AppState::Running { surface } = &mut self.state else {
            println!("Window surface is no prepared");
            return;
        };

        let target_fps = 60.0;
        let frame_duration = Duration::from_secs_f32(1.0 / target_fps);
        let next_frame_time = self.last_frame_time + frame_duration;
        let now = Instant::now();

        if now >= next_frame_time {
            surface.window().request_redraw();
            self.last_frame_time = now;
            event_loop.set_control_flow(event_loop::ControlFlow::Poll);
        } else {
            event_loop.set_control_flow(event_loop::ControlFlow::WaitUntil(next_frame_time));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let AppState::Running { surface } = &mut self.state else {
            unreachable!("got window event while suspended");
        };

        if surface.window().id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let AppState::Running { surface } = &mut self.state else {
                    println!("Window surface is no prepared");
                    return;
                };

                let world = &mut self.world;
                let mut buffer = surface
                    .buffer_mut()
                    .expect("Failed to get to softbuffer buffer");

                let bg = 0xffffffff;
                buffer.fill(bg);

                move_or_bounce_circle(world, &buffer);
                collision(world);
                draw_circle(world, &mut buffer, bg);
                buffer.present().expect("Failed to present buffer");
            }
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZero::new(size.width), NonZero::new(size.height))
                {
                    //resize
                    surface
                        .resize(width, height)
                        .expect("Failed to resize the softbuffer surface");
                }

                //fill color
                let mut buffer = surface
                    .buffer_mut()
                    .expect("Failed to get the softbuffer buffer");

                let color = 0xffffffff;

                buffer.fill(color);
                buffer
                    .present()
                    .expect("Failed to present the softbuffer buffer");
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => self.cursor_position = Some(position),
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                let millis = Duration::from_millis(300);
                let now = Instant::now();

                if now - self.last_mouse_input_time < millis {
                    return;
                }

                match button {
                    MouseButton::Left => {
                        spwan_circle(&mut self.world, self.cursor_position.unwrap());
                    }
                    _ => {}
                }

                self.last_mouse_input_time = now;
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(event_loop::ControlFlow::Poll);

    let context = Context::new(event_loop.owned_display_handle()).unwrap();

    let world = World::new();

    let mut app = App {
        context,
        world,
        cursor_position: None,
        state: AppState::Initial,
        last_frame_time: Instant::now(),
        last_mouse_input_time: Instant::now(),
    };

    event_loop.run_app(&mut app).unwrap();
}

//spawn new circle
pub fn spwan_circle(world: &mut World, p: PhysicalPosition<f64>) {
    let position = Position {
        ox: p.x as f32,
        oy: p.y as f32,
    };
    let velocity = Velocity { vx: 4.0, vy: 4.0 };
    let color = Color::default();
    let radius = Radius { r: 15.0 };

    let entity = world.spawn();

    world
        .add_entity_component(entity, position)
        .add_entity_component(entity, velocity)
        .add_entity_component(entity, color)
        .add_entity_component(entity, radius);
}

// Move circle
pub fn move_or_bounce_circle(
    world: &mut World,
    buffer: &Buffer<'_, OwnedDisplayHandle, Rc<Window>>,
) {
    let width = buffer.width().get() as f32;
    let height = buffer.height().get() as f32;

    if let (Some(p_set), Some(v_set), Some(r_set)) =
        world.get_three_mut_sparse_set::<Position, Velocity, Radius>()
    {
        let shortest_dense: &[Entity] = {
            let dense_arrays: [&[Entity]; 3] =
                [p_set.get_dense(), v_set.get_dense(), r_set.get_dense()];

            &dense_arrays
                .into_iter()
                .min_by_key(|arr| arr.len())
                .unwrap()
                .to_vec()
        };

        for entity in shortest_dense {
            match (
                p_set.get_mut(entity),
                v_set.get_mut(entity),
                r_set.get_mut(entity),
            ) {
                (Some(p), Some(v), Some(r)) => {
                    // bounce
                    let left = p.ox - r.r;
                    let right = p.ox + r.r;
                    let top = p.oy - r.r;
                    let bottom = p.oy + r.r;

                    if left <= 0.0 {
                        v.vx = -v.vx;
                        p.ox = r.r + 1.0;
                    } else if right >= width {
                        v.vx = -v.vx;
                        p.ox = width - r.r - 1.0;
                    }

                    if top <= 0.0 {
                        v.vy = -v.vy;
                        p.oy = r.r + 1.0;
                    } else if bottom >= height {
                        v.vy = -v.vy;
                        p.oy = height - r.r - 1.0
                    }

                    //move
                    p.ox += v.vx;
                    p.oy += v.vy;
                }
                _ => {}
            }
        }
    }

    return;
}

// Collision
pub fn collision(world: &mut World) {
    if let (Some(p_set), Some(v_set), Some(r_set)) =
        world.get_three_mut_sparse_set::<Position, Velocity, Radius>()
    {
        let shortest_dense: &[Entity] = {
            let dense_arrays: [&[Entity]; 3] =
                [p_set.get_dense(), v_set.get_dense(), r_set.get_dense()];

            &dense_arrays
                .into_iter()
                .min_by_key(|arr| arr.len())
                .unwrap()
                .to_vec()
        };

        let len = shortest_dense.len();

        for i in 0..len {
            let e1 = shortest_dense[i];
            for j in (i + 1)..len {
                let e2 = shortest_dense[j];

                // 階段一：不可變讀取 (Immutable Fetch)
                let (p1, p2, r1, r2, v1, v2) = match (
                    p_set.get(&e1),
                    p_set.get(&e2),
                    r_set.get(&e1),
                    r_set.get(&e2),
                    v_set.get(&e1),
                    v_set.get(&e2),
                ) {
                    (Some(p1), Some(p2), Some(r1), Some(r2), Some(v1), Some(v2)) => {
                        (*p1, *p2, *r1, *r2, *v1, *v2)
                    }
                    _ => continue,
                };

                let dx = p2.ox - p1.ox;
                let dy = p2.oy - p1.oy;
                let d2 = dx * dx + dy * dy;
                let min_distance = r1.r + r2.r;

                // 碰撞判定
                if d2 < min_distance * min_distance {
                    let d = d2.sqrt();
                    if d == 0.0 {
                        continue; // 避免除以零，跳過這一對，不要用 return
                    }

                    // 1. 位置修正 (Positional Correction) - 解決黏球問題
                    // 計算重疊深度，並將兩球沿著法向量推開
                    let overlap = min_distance - d;
                    // 每顆球推開一半的重疊量 (假設質量相同)
                    let push_factor = overlap / 2.0;

                    let nx = dx / d; // 法向量 X
                    let ny = dy / d; // 法向量 Y

                    let new_p1_x = p1.ox - nx * push_factor;
                    let new_p1_y = p1.oy - ny * push_factor;
                    let new_p2_x = p2.ox + nx * push_factor;
                    let new_p2_y = p2.oy + ny * push_factor;

                    // 2. 速度計算
                    let tx = -ny; // 切向量 X
                    let ty = nx; // 切向量 Y

                    // 投影到法線與切線上
                    let v1n = v1.vx * nx + v1.vy * ny;
                    let v1t = v1.vx * tx + v1.vy * ty;
                    let v2n = v2.vx * nx + v2.vy * ny;
                    let v2t = v2.vx * tx + v2.vy * ty;

                    // 等質量彈性碰撞：法線速度直接互換
                    let v1n_after = v2n;
                    let v2n_after = v1n;

                    // 合成新速度
                    let new_v1_x = v1n_after * nx + v1t * tx;
                    let new_v1_y = v1n_after * ny + v1t * ty;
                    let new_v2_x = v2n_after * nx + v2t * tx;
                    let new_v2_y = v2n_after * ny + v2t * ty;

                    // 階段二：可變寫入 (Mutable Update)
                    // 這樣寫可以完美避開 Rust 的借用衝突，因為我們是依序取得可變參考
                    if let Some(p) = p_set.get_mut(&e1) {
                        p.ox = new_p1_x;
                        p.oy = new_p1_y;
                    }
                    if let Some(p) = p_set.get_mut(&e2) {
                        p.ox = new_p2_x;
                        p.oy = new_p2_y;
                    }

                    if let Some(v) = v_set.get_mut(&e1) {
                        v.vx = new_v1_x;
                        v.vy = new_v1_y;
                    }
                    if let Some(v) = v_set.get_mut(&e2) {
                        v.vx = new_v2_x;
                        v.vy = new_v2_y;
                    }
                }
            }
        }
    }
}

// Draw circle
pub fn draw_circle(
    world: &mut World,
    buffer: &mut Buffer<'_, OwnedDisplayHandle, Rc<Window>>,
    bg: u32,
) {
    let width = buffer.width().get() as f32;
    let height = buffer.height().get() as f32;

    if let Some(query) = world.query::<(&Position, &Color, &Radius)>() {
        for (_entity, item) in query {
            let (p, c, r) = item;

            let radius = r.r;

            let left = (p.ox - radius).max(0.0) as usize;
            let right = (p.ox + radius).min(width) as usize;
            let top = (p.oy - radius).max(0.0) as usize;
            let bottom = (p.oy + radius).min(height) as usize;

            for y in top..bottom {
                for x in left..right {
                    let dx = x as f32 - p.ox;
                    let dy = y as f32 - p.oy;
                    let d2 = dx * dx + dy * dy;
                    let edge = 0.5f32;

                    if d2 <= radius * radius {
                        let index = y * width as usize + x;
                        let color = c.r << 16 | c.g << 8 | c.b;
                        buffer[index] = anti_alias(edge, d2, radius, color, bg);
                    }
                }
            }
        }
    }
}
pub fn anti_alias(edge: f32, d2: f32, r: f32, color: u32, bg: u32) -> u32 {
    let d = d2.sqrt();
    if d < r - edge {
        return color;
    }

    let alpha = (r - d) / edge;
    let r_color = ((color >> 16) & 0xff) as f32;
    let g_color = ((color >> 8) & 0xff) as f32;
    let b_color = (color & 0xff) as f32;

    let bg_r = ((bg >> 16) & 0xff) as f32;
    let bg_g = ((bg >> 8) & 0xff) as f32;
    let bg_b = (bg & 0xff) as f32;

    let r_out = ((r_color * alpha) + bg_r * (1.0 - alpha)) as u32;
    let g_out = ((g_color * alpha) + bg_g * (1.0 - alpha)) as u32;
    let b_out = ((b_color * alpha) + bg_b * (1.0 - alpha)) as u32;

    (r_out << 16) | (g_out << 8) | b_out
}
