use na::{zero, Matrix4, UnitQuaternion, Vector3};
use specs::prelude::*;

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

        Matrix4::new_nonuniform_scaling(&self.scale)
            * Matrix4::new_rotation(self.rotation.scaled_axis())
            * Matrix4::new_translation(&self.position)
    }

    pub fn translate(&mut self, t: Vector3<f32>) {
        self.position += self.rotation * t;
    }

    pub fn translate_along(&mut self, dir: Vector3<f32>, scaler: f32) {
        if dir != zero() {
            self.position += self.rotation * { dir.normalize() * scaler };
        }
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
            rotation: UnitQuaternion::look_at_rh(&-Vector3::z(), &-Vector3::y()),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
