use mini_ecs::{
    engine::{DeviceInput, Engine, FrameBuffer},
    entity::Entity,
    world::World,
};
use winit::event::MouseButton;

fn main() {
    Engine::new()
        .add_update_system(spwan_circle)
        .add_update_system(move_or_bounce_circle)
        .add_update_system(collision)
        .add_render_system(draw_circle)
        .run();
}

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

//spawn new circle
pub fn spwan_circle(world: &mut World) {
    if let Some(di) = world.get_resource_mut::<DeviceInput>() {
        if !di.mouse_just_pressed(MouseButton::Left) {
            return;
        }

        let (ox, oy) = di.cursor_position();

        let position = Position { ox, oy };
        let velocity = Velocity { vx: 4.0, vy: 4.0 };
        let color = Color {
            r: 255,
            g: 255,
            b: 255,
        };
        let radius = Radius { r: 15.0 };

        let entity = world.spawn();

        world
            .add_entity_component(entity, position)
            .add_entity_component(entity, velocity)
            .add_entity_component(entity, color)
            .add_entity_component(entity, radius);
    }
}

// Move circle
pub fn move_or_bounce_circle(world: &mut World) {
    let width = world.get_resource::<FrameBuffer>().unwrap().width as f32;
    let height = world.get_resource::<FrameBuffer>().unwrap().height as f32;

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
pub fn draw_circle(world: &mut World) {
    // 1. 第一階段：查詢並收集所有需要繪製的圓形資料
    // 這樣 query 的借用範圍只會存在於這個區塊內
    let mut circles = Vec::new();
    if let Some(query) = world.query::<(&Position, &Color, &Radius)>() {
        for (_entity, item) in query {
            let (p, c, r) = item;
            circles.push((*p, *c, *r));
        }
    }

    // 2. 第二階段：安全地獲取 FrameBuffer 的可變借用
    // 此時已經沒有其他人借用 world 了，所以不會報錯
    let fb = world.get_resource_mut::<FrameBuffer>().unwrap();
    let width = fb.width as f32;
    let height = fb.height as f32;
    fb.clear();

    // 3. 進行實際的像素繪製
    for (p, c, r) in circles {
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
                    fb.pixels[index] = anti_alias(edge, d2, radius, color, 0);
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
