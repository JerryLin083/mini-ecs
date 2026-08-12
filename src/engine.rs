use std::{
    collections::HashSet,
    num::NonZero,
    rc::Rc,
    time::{Duration, Instant},
};

use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent},
    event_loop::{self, EventLoop, OwnedDisplayHandle},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

use crate::world::World;

pub type SystemFn = fn(&mut World);

pub struct Engine {
    pub world: World,
    startup_systems: Vec<SystemFn>,
    update_systems: Vec<SystemFn>,
    render_systems: Vec<SystemFn>,
}

impl Engine {
    pub fn new() -> Self {
        let mut world = World::new();

        world.insert_resource(DeviceInput::default());

        Self {
            world,
            startup_systems: Vec::new(),
            update_systems: Vec::new(),
            render_systems: Vec::new(),
        }
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(event_loop::ControlFlow::Poll);

        let context = Context::new(event_loop.owned_display_handle()).unwrap();

        let mut runner = EngineRunner {
            engine: self,
            state: State::Inital,
            context,
            last_frame_time: Instant::now(),
            target_fps: 60.0,
        };

        event_loop.run_app(&mut runner).unwrap();
    }

    pub fn add_startup_system(mut self, system: SystemFn) -> Self {
        self.startup_systems.push(system);
        self
    }

    pub fn add_update_system(mut self, system: SystemFn) -> Self {
        self.update_systems.push(system);
        self
    }

    pub fn add_render_system(mut self, system: SystemFn) -> Self {
        self.render_systems.push(system);
        self
    }
}

enum State {
    Inital,
    Suspended {
        window: Rc<Window>,
    },
    Running {
        surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    },
}

struct EngineRunner {
    engine: Engine,
    state: State,
    context: Context<OwnedDisplayHandle>,
    last_frame_time: Instant,
    target_fps: f32,
}

impl ApplicationHandler for EngineRunner {
    fn new_events(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let StartCause::Init = cause {
            let window = event_loop
                .create_window(WindowAttributes::default())
                .expect("Failed to creating window");

            self.state = State::Suspended {
                window: Rc::new(window),
            }
        }
    }

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let State::Suspended { window } = &mut self.state else {
            unreachable!("got resumed event while not suspended");
        };

        let mut surface =
            Surface::new(&self.context, window.clone()).expect("failed to create surface");

        let size = window.inner_size();
        if let (Some(width), Some(height)) = (NonZero::new(size.width), NonZero::new(size.height)) {
            let _ = surface.resize(width, height);
            self.engine
                .world
                .insert_resource(FrameBuffer::new(size.width, size.height));
        }

        for sys in &self.engine.startup_systems {
            sys(&mut self.engine.world);
        }

        self.state = State::Running { surface };
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let State::Running { surface } = &mut self.state else {
            unreachable!("got resumed event while not running");
        };

        let window = surface.window().clone();
        self.state = State::Suspended { window }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let State::Running { surface } = &mut self.state else {
            println!("Window surface is no prepared");
            return;
        };

        let frame_duration = Duration::from_secs_f32(1.0 / self.target_fps);
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
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(device_input) = self.engine.world.get_resource_mut::<DeviceInput>() {
            device_input.process_event(&event);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let State::Running { surface } = &mut self.state else {
                    unreachable!("got resumed event while not Running");
                };

                if let (Some(width), Some(height)) =
                    (NonZero::new(size.width), NonZero::new(size.height))
                {
                    let _ = surface.resize(width, height);

                    self.engine
                        .world
                        .insert_resource(FrameBuffer::new(size.width, size.height));
                }
            }
            WindowEvent::RedrawRequested => {
                let State::Running { surface } = &mut self.state else {
                    unreachable!("got resumed event while not Running");
                };

                //clear framebuffer
                let fb = self.engine.world.get_resource_mut::<FrameBuffer>().unwrap();
                fb.clear();

                // 1. execute update system
                for sys in &self.engine.update_systems {
                    sys(&mut self.engine.world)
                }

                // 2. execute render system
                for sys in &self.engine.render_systems {
                    sys(&mut self.engine.world)
                }

                // 3. draw framebuffer to softbuffer
                if let Some(fb) = self.engine.world.get_resource::<FrameBuffer>() {
                    if fb.pixels.len() == (fb.width * fb.height) as usize {
                        let mut buffer = surface.buffer_mut().unwrap();
                        buffer.copy_from_slice(&fb.pixels);
                        buffer.present().unwrap()
                    }
                }

                if let Some(di) = self.engine.world.get_resource_mut::<DeviceInput>() {
                    di.update_at_frame_end();
                }
            }
            _ => {}
        }
    }
}

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeviceInput {
    // ===keyboard state===
    keys_held: HashSet<KeyCode>,
    keys_just_pressed: HashSet<KeyCode>,
    keys_just_released: HashSet<KeyCode>,

    // ===mouse state===
    cursor_position: (f32, f32),
    mouse_delta: (f32, f32),
    mouse_buttons_held: HashSet<MouseButton>,
    mouse_buttons_just_pressed: HashSet<MouseButton>,
    mouse_buttons_just_released: HashSet<MouseButton>,
    scroll_delta: (f32, f32),
}

impl DeviceInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_at_frame_end(&mut self) {
        self.keys_just_pressed.clear();
        self.keys_just_released.clear();
        self.mouse_buttons_just_pressed.clear();
        self.mouse_buttons_just_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.scroll_delta = (0.0, 0.0)
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        match event {
            // --- keyboard event ---
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !self.keys_held.contains(&key_code) {
                                self.keys_held.insert(key_code);
                                self.keys_just_pressed.insert(key_code);
                            }
                        }
                        ElementState::Released => {
                            if self.keys_held.remove(&key_code) {
                                self.keys_just_released.insert(key_code);
                            }
                        }
                    }
                }
            }

            // --- cursor move event ---
            WindowEvent::CursorMoved { position, .. } => {
                let new_x = position.x as f32;
                let new_y = position.y as f32;
                self.mouse_delta = (
                    new_x - self.cursor_position.0,
                    new_y - self.cursor_position.1,
                );
                self.cursor_position = (new_x, new_y);
            }

            // --- mouse click event ---
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    if !self.mouse_buttons_held.contains(button) {
                        self.mouse_buttons_held.insert(*button);
                        self.mouse_buttons_just_pressed.insert(*button);
                    }
                }
                ElementState::Released => {
                    if self.mouse_buttons_held.remove(button) {
                        self.mouse_buttons_just_released.insert(*button);
                    }
                }
            },

            // --- scroll event ---
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => self.scroll_delta = (*x, *y),
                MouseScrollDelta::PixelDelta(pos) => {
                    self.scroll_delta = (pos.x as f32, pos.y as f32)
                }
            },

            _ => {}
        }
    }
    pub fn key_held(&self, key: KeyCode) -> bool {
        self.keys_held.contains(&key)
    }

    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.keys_just_pressed.contains(&key)
    }

    pub fn key_just_released(&self, key: KeyCode) -> bool {
        self.keys_just_released.contains(&key)
    }

    // --- 滑鼠查詢 ---
    pub fn cursor_position(&self) -> (f32, f32) {
        self.cursor_position
    }

    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.mouse_buttons_held.contains(&button)
    }

    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_pressed.contains(&button)
    }

    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_just_released.contains(&button)
    }

    pub fn scroll_delta(&self) -> (f32, f32) {
        self.scroll_delta
    }
}
