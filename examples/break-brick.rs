use mini_ecs::{
    engine::{DeviceInput, Engine, FrameBuffer},
    world::World,
};
use winit::keyboard::KeyCode;

fn main() {
    Engine::new()
        .add_startup_system(init_game)
        .add_update_system(move_platform)
        .add_update_system(move_or_bounce_ball)
        .add_render_system(draw_rectangle)
        .add_render_system(draw_ball)
        .run();
}

fn init_game(world: &mut World) {
    let (width, height) = {
        let fb = world.get_resource_mut::<FrameBuffer>().unwrap();

        (fb.width as f32, fb.height as f32)
    };

    // 1. init platform
    let pw = 120f32;
    let ph = 30f32;
    let ox = (width - pw) / 2.0;
    let oy = height - ph - 5.0;
    let pp = Position { ox, oy };
    let ps = Size {
        width: pw,
        height: ph,
    };
    let pc = Color {
        r: 255,
        g: 255,
        b: 255,
    };
    let pv = Velocity { vx: 5.0, vy: 0.0 };

    let entity = world.spawn();
    world
        .add_entity_component(entity, pp)
        .add_entity_component(entity, ps)
        .add_entity_component(entity, pc)
        .add_entity_component(entity, pv);

    // 2. init ball
    let br = Radius { r: 15f32 };
    let ox = (width - br.r) / 2.0;
    let oy = oy - 200.0;
    let bp = Position { ox, oy };
    let bc = Color {
        r: 255,
        g: 255,
        b: 0,
    };
    let bv = Velocity { vx: 1.0, vy: 3.0 };

    let entity = world.spawn();
    world
        .add_entity_component(entity, br)
        .add_entity_component(entity, bp)
        .add_entity_component(entity, bc)
        .add_entity_component(entity, bv);

    // 3. init brick
    let gap = 1.0;
    let brick_row = 5.0;
    let brick_col = 15.0;
    let bkw = (width - gap * (brick_col - 1.0)) / brick_col;
    let bkh = 20.0;
    let bks = Size {
        width: bkw,
        height: bkh,
    };
    let bkc = Color {
        r: 200,
        g: 50,
        b: 120,
    };

    for row in 0..brick_row as usize {
        for col in 0..brick_col as usize {
            let ox = (gap + bkw) * col as f32;
            let oy = (gap + bkh) * row as f32;
            let bkp = Position { ox, oy };
            let entity = world.spawn();
            world
                .add_entity_component(entity, bks)
                .add_entity_component(entity, bkc)
                .add_entity_component(entity, bkp);
        }
    }
}

fn move_platform(world: &mut World) {
    let fb = world.get_resource::<FrameBuffer>().unwrap();
    let width = fb.width;

    let key_code;
    let di = world.get_resource::<DeviceInput>().unwrap();
    if di.key_held(KeyCode::ArrowLeft) {
        key_code = KeyCode::ArrowLeft;
    } else if di.key_held(KeyCode::ArrowRight) {
        key_code = KeyCode::ArrowRight;
    } else {
        return;
    }

    if let Some(query) = world.query::<(&mut Position, &Size, &Velocity)>() {
        for (_entity, item) in query {
            let (p, s, v) = item;

            if key_code == KeyCode::ArrowLeft {
                p.ox = (p.ox - v.vx).max(0.0);
            } else {
                p.ox = (p.ox + v.vx).min(width as f32 - s.width);
            }
        }
    }
}

fn draw_rectangle(world: &mut World) {
    let mut rects = Vec::new();
    if let Some(query) = world.query::<(&Position, &Size, &Color)>() {
        for (_entity, item) in query {
            let (p, s, c) = item;
            rects.push((*p, *s, *c));
        }
    }

    let fb = world.get_resource_mut::<FrameBuffer>().unwrap();
    let width = fb.width as f32;
    let height = fb.height as f32;

    for (p, s, c) in rects {
        let left = p.ox.max(0.0) as usize;
        let right = (p.ox + s.width).min(width) as usize;
        let top = p.oy.max(0.0) as usize;
        let bottom = (p.oy + s.height).min(height) as usize;

        for y in top..bottom {
            for x in left..right {
                let idx = y * width as usize + x;
                fb.pixels[idx] = c.r << 16 | c.g << 8 | c.b;
            }
        }
    }
}

fn move_or_bounce_ball(world: &mut World) {
    let (width, height) = {
        let fb = world.get_resource::<FrameBuffer>().unwrap();
        (fb.width as f32, fb.height as f32)
    };

    if let Some(query) = world.query::<(&mut Position, &mut Velocity, &Radius)>() {
        for (_entity, item) in query {
            let (p, v, r) = item;
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
                p.oy = height - r.r - 1.0;
            }

            //move
            p.ox += v.vx;
            p.oy += v.vy;
        }
    }
}

fn draw_ball(world: &mut World) {
    let mut balls = Vec::new();

    if let Some(query) = world.query::<(&mut Position, &Color, &Radius)>() {
        for (_entity, item) in query {
            let (p, c, r) = item;
            balls.push((*p, *c, *r));
        }
    }

    let fb = world.get_resource_mut::<FrameBuffer>().unwrap();
    let width = fb.width as f32;
    let height = fb.height as f32;

    for (p, c, r) in balls {
        let left = (p.ox - r.r).max(0.0) as usize;
        let right = (p.ox + r.r).min(width) as usize;
        let top = (p.oy - r.r).max(0.0) as usize;
        let bottom = (p.oy + r.r).min(height) as usize;
        let edge = 0.5f32;

        for y in top..bottom {
            for x in left..right {
                let dx = x as f32 - p.ox;
                let dy = y as f32 - p.oy;
                let d2 = dx * dx + dy * dy;

                if d2 <= r.r * r.r {
                    let idx = y * width as usize + x;
                    let color = c.r << 16 | c.g << 8 | c.b;
                    fb.pixels[idx] = anti_alias(edge, d2, r.r, color, 0);
                }
            }
        }
    }
}

// ===Helper===
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

#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub ox: f32,
    pub oy: f32,
}
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u32,
    pub g: u32,
    pub b: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Radius {
    pub r: f32,
}
