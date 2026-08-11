use std::{
    num::NonZero,
    rc::Rc,
    time::{Duration, Instant},
};

use softbuffer::{Context, Surface};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{self, EventLoop, OwnedDisplayHandle},
    window::{Window, WindowAttributes},
};

use crate::world::World;

pub type SystemFn = fn(&mut World);

pub struct Engine {
    pub world: World,
    update_systems: Vec<SystemFn>,
    render_systems: Vec<SystemFn>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            world: World::new(),
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

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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

        self.state = State::Running { surface };
    }

    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
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
