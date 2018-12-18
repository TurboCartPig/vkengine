mod transform;

pub use crate::systems::transform::TransformSystem;

use crate::{
    components::Transform,
    renderer::{camera::ActiveCamera, RenderEvent, RenderEvents},
    resources::{
        ControllerAxis, ControllerEvent, ControllerEvents, FocusGained, KeyboardEvent,
        KeyboardEvents, Keycode, MouseEvent, MouseEvents, ShouldClose, Time,
    },
};
use float_duration::TimePoint;
use log::info;
use nalgebra::{UnitQuaternion, Vector3};
use sdl2::{
    controller::GameController,
    event::{Event, WindowEvent},
    video::Window as SdlWindow,
    EventPump, GameControllerSubsystem, Sdl, VideoSubsystem,
};
use shrev::ReaderId;
use specs::prelude::*;
use std::{
    mem,
    ops::{AddAssign, SubAssign},
    time::Instant,
};

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

        let delta = now
            .float_duration_since(self.last_frame)
            .unwrap()
            .as_seconds() as f32;
        let first_frame = now
            .float_duration_since(self.first_frame)
            .unwrap()
            .as_seconds() as f32;

        *time = Time::new(first_frame, delta, time.timescale());

        mem::replace(&mut self.last_frame, now);
    }
}

#[derive(Debug, Default)]
pub struct Axis {
    value: f32,
}

impl Axis {
    pub fn set(&mut self, value: f32) {
        let value = if value > 1. {
            1.
        } else if value < -1. {
            -1.
        } else {
            value
        };

        self.value = value;
    }

    pub fn get(&self) -> f32 {
        self.value
    }
}

impl AddAssign<f32> for Axis {
    fn add_assign(&mut self, value: f32) {
        self.value = if self.value + value > 1. {
            1.
        } else if self.value + value < -1. {
            -1.
        } else {
            self.value + value
        }
    }
}

impl SubAssign<f32> for Axis {
    fn sub_assign(&mut self, value: f32) {
        self.value = if self.value - value > 1. {
            1.
        } else if self.value - value < -1. {
            -1.
        } else {
            self.value - value
        }
    }
}

//TODO Decide if this or events is the best option for input
#[derive(Debug, Default)]
pub struct GameInput {
    forward: Axis,
    right: Axis,
    controller_view_hor: Axis,
    controller_view_ver: Axis,
    mouse_view_hor: f32,
    mouse_view_ver: f32,
}

impl GameInput {
    pub fn view(&self) -> (f32, f32) {
        (self.controller_view_hor.get() + self.mouse_view_hor, self.controller_view_ver.get() + self.mouse_view_ver)
    }
}

/// Turns keyboard events into game data
#[derive(Debug, Default)]
pub struct GameInputSystem {
    keyboard_read_id: Option<ReaderId<KeyboardEvent>>,
    mouse_read_id: Option<ReaderId<MouseEvent>>,
    controller_read_id: Option<ReaderId<ControllerEvent>>,
}

impl<'a> System<'a> for GameInputSystem {
    type SystemData = (
        Write<'a, GameInput>,
        Write<'a, ShouldClose>,
        Read<'a, KeyboardEvents>,
        Read<'a, MouseEvents>,
        Read<'a, ControllerEvents>,
    );

    fn run(
        &mut self,
        (mut input, mut should_close, keyboard_events, mouse_events, controller_events): Self::SystemData,
    ) {
        // Handle controller event
        // -----------------------------------------------------------------------------------------------------
        controller_events
            .read(self.controller_read_id.as_mut().unwrap())
            .for_each(|event| {
                match event {
                    ControllerEvent::AxisMotion { axis, value, .. } => match axis {
                        ControllerAxis::LeftX => input.right.set(*value),
                        ControllerAxis::LeftY => input.forward.set(-value),
                        ControllerAxis::RightX => input.controller_view_hor.set(*value),
                        ControllerAxis::RightY => input.controller_view_ver.set(*value),
                        _ => (),
                    },
                    _ => (),
                }
            });

        // Handle keyboard events
        // -----------------------------------------------------------------------------------------------------
        keyboard_events
            .read(self.keyboard_read_id.as_mut().unwrap())
            .for_each(|event| match event {
                // Quit the game with q
                KeyboardEvent {
                    pressed: true,
                    keycode: Keycode::Q,
                    ..
                } => {
                    should_close.0 = true;
                }
                KeyboardEvent {
                    pressed: true,
                    keycode,
                    ..
                } => match keycode {
                    Keycode::W => input.forward.set(1.),
                    Keycode::S => input.forward.set(-1.),
                    Keycode::D => input.right.set(1.),
                    Keycode::A => input.right.set(-1.),
                    _ => (),
                },
                KeyboardEvent {
                    pressed: false,
                    keycode,
                    ..
                } => match keycode {
                    Keycode::W => input.forward.set(0.),
                    Keycode::S => input.forward.set(0.),
                    Keycode::D => input.right.set(0.),
                    Keycode::A => input.right.set(0.),
                    _ => (),
                },
            });

        // Handle mouse events
        // -----------------------------------------------------------------------------------------------------
        input.mouse_view_ver = 0.;
        input.mouse_view_hor = 0.;

        mouse_events
            .read(self.mouse_read_id.as_mut().unwrap())
            .for_each(|event| match event {
                MouseEvent::Motion { delta, .. } => {
                    input.mouse_view_hor += delta.0 as f32;
                    input.mouse_view_ver += delta.1 as f32;
                }
                _ => (),
            });
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);

        // Register keyboard event reader
        let mut keyboard = res.fetch_mut::<KeyboardEvents>();
        let reader_id = keyboard.register_reader();
        self.keyboard_read_id = Some(reader_id);

        // Regster mouse event reader
        let mut mouse = res.fetch_mut::<MouseEvents>();
        let reader_id = mouse.register_reader();
        self.mouse_read_id = Some(reader_id);

        // Register controller event reader
        let mut controller = res.fetch_mut::<ControllerEvents>();
        let reader_id = controller.register_reader();
        self.controller_read_id = Some(reader_id);
    }
}

/// Fly control system
pub struct FlyControlSystem;

impl<'a> System<'a> for FlyControlSystem {
    type SystemData = (
        Read<'a, Time>,
        Read<'a, FocusGained>,
        Read<'a, GameInput>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (time, input_enabled, input, active_camera, mut transform): Self::SystemData,
    ) {
        // Only handle input if the window is focused
        if !input_enabled.0 {
            return;
        }

        // Get the camera transform
        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        // Rotation
        // ------------------------------------------------------------------------------------------------------------
        let (yaw, pitch) = input.view();
        let (yaw, pitch) = (yaw * -0.001, pitch * -0.001);

        camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        // Translation
        // ------------------------------------------------------------------------------------------------------------
        camera_t.translate_forward(input.forward.get() * time.delta() as f32);
        camera_t.translate_right(input.right.get() * time.delta() as f32);
    }
}

// pub struct SendSyncWindow(pub SdlWindow);

// unsafe impl Send for SendSyncWindow {}
// unsafe impl Sync for SendSyncWindow {}

static LEFT_THUMB_DEADZONE: i16 = 7849;
static RIGHT_THUMB_DEADZONE: i16 = 8689;
static TRIGGER_THRESHOLD: i16 = 30;

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
        let controllers = Vec::with_capacity(4);
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

// FIXME Fullscreen currently crashes in forign code
impl<'a> System<'a> for SDLSystem {
    type SystemData = (
        Write<'a, ShouldClose>,
        Write<'a, FocusGained>,
        Write<'a, RenderEvents>,
        Write<'a, KeyboardEvents>,
        Write<'a, MouseEvents>,
        Write<'a, ControllerEvents>,
    );

    fn run(
        &mut self,
        (
            mut should_close,
            mut window_focus,
            mut render_events,
            mut keyboard_events,
            mut mouse_events,
            mut controller_events,
        ): Self::SystemData,
    ) {
        let mouse_util = &self.context.mouse();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => should_close.0 = true,
                // Window event
                // ---------------------------------------------------------------------------------------------------------------
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
                    }
                    WindowEvent::Resized(_, _) => {
                        render_events.single_write(RenderEvent::WindowResized);
                    }
                    _ => (),
                },
                // Mouse event
                // ---------------------------------------------------------------------------------------------------------------
                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    let event = MouseEvent::Motion {
                        delta: (xrel, yrel),
                        absolute: (x, y),
                    };

                    mouse_events.single_write(event);
                }
                Event::MouseButtonDown {
                    mouse_btn, clicks, ..
                } => {
                    let event = MouseEvent::Button {
                        pressed: true,
                        button: mouse_btn,
                        clicks,
                    };

                    mouse_events.single_write(event);
                }
                Event::MouseButtonUp {
                    mouse_btn, clicks, ..
                } => {
                    let event = MouseEvent::Button {
                        pressed: false,
                        button: mouse_btn,
                        clicks,
                    };

                    mouse_events.single_write(event);
                }
                Event::MouseWheel { x, y, .. } => {
                    let event = MouseEvent::Wheel { x, y };

                    mouse_events.single_write(event);
                }
                // Keyboard event
                // ---------------------------------------------------------------------------------------------------------------
                Event::KeyDown {
                    keycode: Some(keycode),
                    keymod,
                    repeat,
                    ..
                } => {
                    let event = KeyboardEvent {
                        pressed: true,
                        keycode,
                        keymod,
                        repeat,
                    };

                    keyboard_events.single_write(event);
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    keymod,
                    repeat,
                    ..
                } => {
                    let event = KeyboardEvent {
                        pressed: false,
                        keycode,
                        keymod,
                        repeat,
                    };

                    keyboard_events.single_write(event);
                }
                // Controller event
                // ---------------------------------------------------------------------------------------------------------------
                Event::ControllerDeviceAdded { which, .. } => {
                    let name = self.controller_subsystem.name_for_index(which).unwrap();
                    info!("Found game controller: {}", name);

                    let controller = self.controller_subsystem.open(which).unwrap();
                    self.controllers.insert(which as usize, controller);

                    let event = ControllerEvent::Connected(which as i32);
                    controller_events.single_write(event);
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    let name = self
                        .controller_subsystem
                        .name_for_index(which as u32)
                        .unwrap();
                    info!("Game controller removed: {}", name);

                    self.controllers.remove(which as usize);

                    let event = ControllerEvent::Disconnected(which);
                    controller_events.single_write(event);
                }
                Event::ControllerAxisMotion {
                    which, axis, value, ..
                } => {
                    // If the value is inside deadzone: then value is 0
                    let value = match axis {
                        // Left
                        ControllerAxis::LeftX | ControllerAxis::LeftY => {
                            if value > LEFT_THUMB_DEADZONE || value < -LEFT_THUMB_DEADZONE {
                                value
                            } else {
                                0
                            }
                        }
                        // Right
                        ControllerAxis::RightX | ControllerAxis::RightY => {
                            if value > RIGHT_THUMB_DEADZONE || value < -RIGHT_THUMB_DEADZONE {
                                value
                            } else {
                                0
                            }
                        }
                        // Triggers
                        ControllerAxis::TriggerLeft | ControllerAxis::TriggerRight => {
                            if value > TRIGGER_THRESHOLD {
                                value
                            } else {
                                0
                            }
                        }
                    };

                    // Normalize
                    let value = value as f32 / std::i16::MAX as f32;

                    let event = ControllerEvent::AxisMotion {
                        id: which,
                        axis,
                        value,
                    };

                    controller_events.single_write(event);
                }
                Event::ControllerButtonDown { which, button, .. } => {
                    let event = ControllerEvent::Button {
                        id: which,
                        pressed: true,
                        button,
                    };

                    controller_events.single_write(event);
                }
                Event::ControllerButtonUp { which, button, .. } => {
                    let event = ControllerEvent::Button {
                        id: which,
                        pressed: false,
                        button,
                    };

                    controller_events.single_write(event);
                }
                _ => (),
            }
        }
    }
}
