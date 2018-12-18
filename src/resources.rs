use sdl2::keyboard::Mod;
use shrev::EventChannel;
use std::ops::{Deref, DerefMut};

pub use sdl2::{
    controller::{Axis as ControllerAxis, Button as ControllerButton},
    keyboard::Keycode,
    mouse::MouseButton,
};

/// Resource for accessing delta time
#[derive(Debug)]
pub struct Time {
    pub first_frame: f32,
    delta: f32,
    timescale: f32,
}

impl Time {
    pub fn new(first_frame: f32, delta: f32, timescale: f32) -> Self {
        Self {
            first_frame,
            delta,
            timescale,
        }
    }

    pub fn delta(&self) -> f32 {
        self.delta * self.timescale
    }

    pub fn timescale(&self) -> f32 {
        self.timescale
    }
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta: 1.,
            first_frame: 0.,
            timescale: 1.,
        }
    }
}

/// Resource for signaling that the user has asked to close the game
#[derive(Debug, Default)]
pub struct ShouldClose(pub bool);

/// Is the main window focused?
#[derive(Debug, Default)]
pub struct FocusGained(pub bool);

#[derive(Debug)]
pub struct KeyboardEvent {
    pub pressed: bool,
    pub keycode: Keycode,
    pub keymod: Mod,
    pub repeat: bool,
}

#[derive(Default)]
pub struct KeyboardEvents(EventChannel<KeyboardEvent>);

impl Deref for KeyboardEvents {
    type Target = EventChannel<KeyboardEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KeyboardEvents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub enum MouseEvent {
    Button {
        pressed: bool,
        button: MouseButton,
        clicks: u8,
    },
    Wheel {
        x: i32,
        y: i32,
    },
    Motion {
        delta: (i32, i32),
        absolute: (i32, i32),
    },
}

#[derive(Default)]
pub struct MouseEvents(EventChannel<MouseEvent>);

impl Deref for MouseEvents {
    type Target = EventChannel<MouseEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MouseEvents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub enum ControllerEvent {
    Connected(i32),
    Disconnected(i32),
    AxisMotion {
        id: i32,
        axis: ControllerAxis,
        value: i16,
    },
    Button {
        id: i32,
        pressed: bool,
        button: ControllerButton,
    },
}

#[derive(Default)]
pub struct ControllerEvents(EventChannel<ControllerEvent>);

impl Deref for ControllerEvents {
    type Target = EventChannel<ControllerEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ControllerEvents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
