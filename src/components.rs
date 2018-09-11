use na::{Point3, Vector3};
use specs::prelude::*;

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Transform {
    // Offset from origin
    pub position: Vector3<f32>,
    // Roll, pitch, yaw
    pub rotation: (f32, f32, f32),
    // Nonuniform scale
    pub scale: Vector3<f32>,
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
