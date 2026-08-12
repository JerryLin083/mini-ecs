use mini_ecs::{
    engine::{DeviceInput, Engine, FrameBuffer},
    world::World,
};
use winit::keyboard::KeyCode;

fn main() {
    Engine::new()
        .add_startup_system(init_game)
        .add_update_system(move_platform)
        .add_render_system(draw_rectangle)
        .run();
}

fn init_game(world: &mut World) {
    let (width, height) = {
        let fb = world.get_resource_mut::<FrameBuffer>().unwrap();

        (fb.width as f32, fb.height as f32)
    };

    // 1. init platform
    let pw = width / 10.0;
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
    let pv = Velocity { vx: 5.0, yx: 0.0 };

    let entity = world.spawn();
    world
        .add_entity_component(entity, pp)
        .add_entity_component(entity, ps)
        .add_entity_component(entity, pc)
        .add_entity_component(entity, pv);

    // 2. init ball

    // 3. init brick
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
            let (p, _, v) = item;

            if key_code == KeyCode::ArrowLeft {
                p.ox = (p.ox - v.vx).max(0.0);
            } else {
                p.ox = (p.ox + v.vx).min(width as f32);
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
    pub yx: f32,
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
