use std::collections::HashMap;

pub use sdl2::{keyboard::Keycode, mouse::MouseButton};

/// Resource for accessing delta time
#[derive(Debug)]
pub struct Time {
    pub delta: f64,
    pub first_frame: f64,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta: 1.,
            first_frame: 0f64,
        }
    }
}

/// Resource for signaling that the user has asked to close the game
#[derive(Debug, Default)]
pub struct ShouldClose(pub bool);

/// Is the main window focused?
#[derive(Debug, Default)]
pub struct FocusGained(pub bool);

#[derive(Debug, Default)]
pub struct Keyboard {
    keys: HashMap<Keycode, bool>,
}

impl Keyboard {
    pub fn clear_all(&mut self) {
        self.keys.clear();
    }

    pub fn pressed(&self, key: Keycode) -> bool {
        *self.keys.get(&key).unwrap_or(&false)
    }

    pub fn set_pressed(&mut self, key: Keycode, val: bool) {
        self.keys.insert(key, val);
    }
}

#[derive(Debug, Default)]
pub struct Mouse {
    buttons: HashMap<MouseButton, bool>,
    pub absolute: (i32, i32),
    pub delta: (i32, i32),
}

impl Mouse {
    pub fn clear_deltas(&mut self) {
        self.delta = (0, 0)
    }

    pub fn clear_all(&mut self) {
        self.buttons.clear();
        self.absolute = (0, 0);
        self.delta = (0, 0);
    }

    pub fn pressed(&self, button: MouseButton) -> bool {
        *self.buttons.get(&button).unwrap_or(&false)
    }

    pub fn set_pressed(&mut self, button: MouseButton, val: bool) {
        self.buttons.insert(button, val);
    }
}
