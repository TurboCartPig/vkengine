use std::collections::HashMap;
use na::{Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3};
use specs::prelude::*;
use winit::VirtualKeyCode;

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Transform {
    // Offset from origin
    pub position: Vector3<f32>,
    // Roll, pitch, yaw
    pub rotation: (f32, f32, f32),
    // Vector rotation
    //pub rotation: Vector<f32>,
    // Nonuniform scale
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn as_matrix(&self) -> Matrix4<f32> {
        let mut matrix = Isometry3::identity();

        matrix.append_translation_mut(&Translation3::from_vector(self.position));

        matrix.append_rotation_wrt_center_mut(&UnitQuaternion::from_euler_angles(
            self.rotation.0,
            self.rotation.1,
            self.rotation.2,
        ));

        let matrix = matrix
            .to_homogeneous()
            .prepend_nonuniform_scaling(&self.scale);

        matrix
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: (0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

pub struct Keyboard {
    pub pressed: HashMap<VirtualKeyCode, bool>,
}

impl Keyboard {
    pub fn pressed(&self, key: VirtualKeyCode) -> bool {
        match self.pressed.get(&key) {
            Some(true) => true,
            _ => false
        }
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            pressed: HashMap::new(),
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
