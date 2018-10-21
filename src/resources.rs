use winit::VirtualKeyCode;

// TODO Use scancodes instead of virtual key codes
// 170 is the number of variants as of winit 0.17.2
pub struct Keyboard {
    pressed: [bool; 170],
}

impl Keyboard {
    pub fn release(&mut self, key: VirtualKeyCode) {
        self.pressed[key as usize] = false;
    }

    pub fn press(&mut self, key: VirtualKeyCode) {
        self.pressed[key as usize] = true;
    }

    pub fn pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed[key as usize]
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            pressed: [false; 170],
        }
    }
}

// TODO Add Mouse buttons
// TODO Consider moving grabbed
pub struct Mouse {
    pub move_delta: (f64, f64),
    pub scroll_delta: (f32, f32),
    pub grabbed: bool,
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

pub struct DeltaTime {
    pub delta: f64,
    pub first_frame: f64,
}

impl Default for DeltaTime {
    fn default() -> Self {
        DeltaTime {
            delta: 1f64,
            first_frame: 0f64,
        }
    }
}

pub struct ShouldClose(pub bool);

impl Default for ShouldClose {
    fn default() -> Self {
        ShouldClose(false)
    }
}
