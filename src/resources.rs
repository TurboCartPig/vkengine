use winit::VirtualKeyCode;
use hibitset::BitSet;

// TODO Use scancodes instead of virtual key codes
pub struct Keyboard {
    pressed: BitSet,
}

impl Keyboard {
    pub fn release(&mut self, key: VirtualKeyCode) {
        self.pressed.remove(key as u32);
    }

    pub fn press(&mut self, key: VirtualKeyCode) {
        self.pressed.add(key as u32);
    }

    pub fn pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed.contains(key as u32)
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            pressed: BitSet::new(),
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
