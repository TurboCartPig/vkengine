use fixedbitset::FixedBitSet;
use gilrs::Button;
use nalgebra::{clamp, Vector2};
use winit::VirtualKeyCode;

/// Resource for keeping track of which keys are pressed
///
/// # Panics
///
/// - If the key as u32 > pressed.len()
pub struct Keyboard {
    pressed: FixedBitSet,
}

impl Keyboard {
    pub fn release(&mut self, key: VirtualKeyCode) {
        self.pressed.set(key as usize, false);
    }

    pub fn press(&mut self, key: VirtualKeyCode) {
        self.pressed.set(key as usize, true);
    }

    pub fn pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed.contains(key as usize)
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            pressed: FixedBitSet::with_capacity(VirtualKeyCode::Cut as usize),
        }
    }
}

/// Resource for keeping track of movement deltas and button states of a mouse
// TODO Add Mouse buttons
// TODO Consider making grabbed its own resource
pub struct Mouse {
    pub move_delta: (f64, f64),
    pub scroll_delta: (f32, f32),
    pub grabbed: bool,
}

impl Mouse {
    pub fn clear_deltas(&mut self) {
        self.move_delta = (0.0, 0.0);
        self.scroll_delta = (0.0, 0.0);
    }
}

impl Default for Mouse {
    fn default() -> Self {
        Self {
            move_delta: (0.0, 0.0),
            scroll_delta: (0.0, 0.0),
            grabbed: true,
        }
    }
}

/// Linear scale between 1 and -1
#[derive(Default, Debug)]
pub struct Axis {
    value: f32,
}

impl Axis {
    pub fn delta(&mut self, delta: f32) {
        // self.value = clamp(self.value + delta, -1., 1.);
        self.value = clamp(delta, -1., 1.);
        // println!("Axis value: {}", self.value);
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}

/// A stick is two axis
#[derive(Default, Debug)]
pub struct Stick {
    pub x: Axis,
    pub y: Axis,
}

impl Stick {
    pub fn to_vector(&self) -> Vector2<f32> {
        Vector2::new(self.x.value(), self.y.value()).normalize()
    }
}

/// Generic gamepad
pub struct Gamepad {
    pub left: Stick,
    pub right: Stick,
    pub lbumper: Axis,
    pub rbumper: Axis,
    buttons: FixedBitSet,
}

impl Gamepad {
    pub fn pressed(&self, button: Button) -> bool {
        self.buttons.contains(button as usize)
    }

    pub fn press(&mut self, button: Button) {
        self.buttons.set(button as usize, true);
    }

    pub fn release(&mut self, button: Button) {
        self.buttons.set(button as usize, false);
    }
}

// FIXME Hardcoded capacity
impl Default for Gamepad {
    fn default() -> Self {
        Self {
            left: Stick::default(),
            right: Stick::default(),
            lbumper: Axis::default(),
            rbumper: Axis::default(),
            buttons: FixedBitSet::with_capacity(20usize),
        }
    }
}

/// Resource for accessing delta time
pub struct Time {
    pub delta: f64,
    pub first_frame: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta: 1f64,
            first_frame: 0f64,
        }
    }
}

/// Resource for signaling that the user has asked to close the game
pub struct ShouldClose(pub bool);

impl Default for ShouldClose {
    fn default() -> Self {
        ShouldClose(false)
    }
}
