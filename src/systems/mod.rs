mod transform;

pub use crate::systems::transform::TransformSystem;

use crate::{
    components::Transform,
    renderer::{camera::ActiveCamera, RenderEvent, RenderEvents},
    resources::{FocusGained, Keyboard, Keycode, Mouse, ShouldClose, Time},
};
use float_duration::TimePoint;
use log::info;
use nalgebra::{UnitQuaternion, Vector3};
use sdl2::{
    controller::GameController,
    event::{Event, WindowEvent},
    video::{FullscreenType, Window as SdlWindow},
    EventPump, GameControllerSubsystem, Sdl, VideoSubsystem,
};
use specs::prelude::*;
use std::{mem, time::Instant};

/// A System for updating the Time resource in order to expose things like delta time
pub struct TimeSystem {
    first_frame: Instant,
    last_frame: Instant,
}

impl Default for TimeSystem {
    fn default() -> Self {
        TimeSystem {
            first_frame: Instant::now(),
            last_frame: Instant::now(),
        }
    }
}

impl<'a> System<'a> for TimeSystem {
    type SystemData = Write<'a, Time>;

    fn run(&mut self, mut time: Self::SystemData) {
        let now = Instant::now();

        let delta = now.float_duration_since(self.last_frame).unwrap();
        time.delta = delta.as_seconds();

        let first_frame = now.float_duration_since(self.first_frame).unwrap();
        time.first_frame = first_frame.as_seconds();

        mem::replace(&mut self.last_frame, now);
    }
}

/// Fly control system
pub struct FlyControlSystem;

impl<'a> System<'a> for FlyControlSystem {
    type SystemData = (
        Read<'a, Time>,
        Read<'a, FocusGained>,
        Read<'a, Keyboard>,
        Read<'a, Mouse>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (time, input_enabled, keyboard, mouse, active_camera, mut transform): Self::SystemData,
    ) {
        // Only handle input if the window is focused
        // ------------------------------------------------------------------------------------------------------------
        if !input_enabled.0 {
            return;
        }

        // Get the camera transform
        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        // Rotation
        // ------------------------------------------------------------------------------------------------------------
        let yaw = -mouse.delta.0 as f32;
        let pitch = -mouse.delta.1 as f32;
        // Input scaling
        let (yaw, pitch) = (yaw * 0.001, pitch * 0.001);

        camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        // Translation
        // ------------------------------------------------------------------------------------------------------------
        if keyboard.pressed(Keycode::W) {
            camera_t.translate_forward(1.0 * time.delta as f32);
        }

        if keyboard.pressed(Keycode::S) {
            camera_t.translate_forward(-1.0 * time.delta as f32);
        }

        if keyboard.pressed(Keycode::A) {
            camera_t.translate_right(-1.0 * time.delta as f32);
        }

        if keyboard.pressed(Keycode::D) {
            camera_t.translate_right(1.0 * time.delta as f32);
        }
    }
}

// pub struct SendSyncWindow(pub SdlWindow);

// unsafe impl Send for SendSyncWindow {}
// unsafe impl Sync for SendSyncWindow {}

/// System for turning sdl events into ecs data
pub struct SDLSystem {
    context: Sdl,
    _video_subsystem: VideoSubsystem,
    window: SdlWindow,
    controller_subsystem: GameControllerSubsystem,
    controllers: Vec<GameController>,
    event_pump: EventPump,
}

impl SDLSystem {
    pub fn new() -> Self {
        let context = sdl2::init().unwrap();
        let _video_subsystem = context.video().unwrap();
        let controller_subsystem = context.game_controller().unwrap();
        let controllers = Vec::new();
        let event_pump = context.event_pump().unwrap();
        
        context.mouse().set_relative_mouse_mode(true);
        
        let window = _video_subsystem
            .window("vkengine", 1600, 900)
            .resizable()
            .position_centered()
            .input_grabbed()
            .allow_highdpi()
            .vulkan()
            .build()
            .unwrap();

        Self {
            context,
            _video_subsystem,
            window,
            controller_subsystem,
            controllers,
            event_pump,
        }
    }

    pub fn window(&self) -> &SdlWindow {
        &self.window
    }
}

impl<'a> System<'a> for SDLSystem {
    type SystemData = (
        Write<'a, ShouldClose>,
        Write<'a, FocusGained>,
        Write<'a, Keyboard>,
        Write<'a, Mouse>,
        Write<'a, RenderEvents>,
    );

    fn run(
        &mut self,
        (mut should_close, mut window_focus, mut keyboard, mut mouse, mut render_events): Self::SystemData,
    ) {
        // Reset
        mouse.clear_deltas();

        let mouse_util = &self.context.mouse();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => should_close.0 = true,
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::FocusGained => {
                        window_focus.0 = true;
                        mouse_util.capture(true);
                        mouse_util.show_cursor(false);
                    }
                    WindowEvent::FocusLost => {
                        window_focus.0 = false;
                        mouse_util.capture(false);
                        mouse_util.show_cursor(true);

                        mouse.clear_all();
                        keyboard.clear_all();
                    }
                    _ => (),
                },
                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    mouse.absolute = (x, y);
                    mouse.delta = (xrel, yrel);
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    mouse.set_pressed(mouse_btn, true);
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    mouse.set_pressed(mouse_btn, false);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    ..
                } => {
                    should_close.0 = true;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F),
                    ..
                } => {
                    self.window.set_fullscreen(FullscreenType::Desktop).unwrap();
                    render_events.channel.single_write(RenderEvent::EnterFullscreen);
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    keyboard.set_pressed(key, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    keyboard.set_pressed(key, false);
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    let name = self.controller_subsystem.name_for_index(which).unwrap();
                    info!("Found game controller: {}", name);

                    let controller = self.controller_subsystem.open(which).unwrap();
                    self.controllers.push(controller);
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    // Find index of controller to remove
                    let idx = self
                        .controllers
                        .iter()
                        .enumerate()
                        .find(|(_, c)| c.instance_id() == which)
                        .map(|(idx, _)| idx)
                        .unwrap();
                    self.controllers.remove(idx);
                }
                _ => (),
            }
        }
    }
}
