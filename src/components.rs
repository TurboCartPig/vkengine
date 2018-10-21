use na::{Matrix4, UnitQuaternion, Vector3};
use specs::prelude::*;
use winit::VirtualKeyCode;

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Transform {
    // Offset from origin
    pub position: Vector3<f32>,
    // Roll, pitch, yaw
    pub rotation: UnitQuaternion<f32>,
    // Nonuniform scale
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn as_matrix(&self) -> Matrix4<f32> {
        // let mut matrix = Isometry3::identity();
        
        // matrix.append_rotation_mut(&self.rotation);

        // matrix.append_translation_mut(&Translation3::from_vector(self.position));
        
        // let matrix = matrix
        //     .to_homogeneous()
        //     .prepend_nonuniform_scaling(&self.scale);

        // matrix
        
        Matrix4::new_nonuniform_scaling(&self.scale) * Matrix4::new_rotation(self.rotation.scaled_axis()) * Matrix4::new_translation(&self.position)
    }

    pub fn translate(&mut self, t: Vector3<f32>) {
        self.position += self.rotation * t;
    }

    pub fn translate_forward(&mut self, s: f32) {
        self.translate(Vector3::new(0.0, 0.0, s))
    }

    pub fn translate_right(&mut self, s: f32) {
        self.translate(Vector3::new(-s, 0.0, 0.0))
    }

    pub fn rotate_global(&mut self, r: UnitQuaternion<f32>) {
        self.rotation = r * self.rotation;
    }

    pub fn rotate_local(&mut self, r: UnitQuaternion<f32>) {
        self.rotation = self.rotation * r;
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: UnitQuaternion::from_euler_angles(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

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
