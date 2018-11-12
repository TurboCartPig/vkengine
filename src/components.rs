use na::{zero, Matrix4, UnitQuaternion, Vector3, Vector4};
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
        Matrix4::new_nonuniform_scaling(&self.scale)
            * Matrix4::new_rotation(self.rotation.scaled_axis())
            * Matrix4::new_translation(&self.position)

        // Matrix4::from_columns(&[
        //     Vector4::new(),
        //     Vector4::new(),
        //     Vector4::new(),
        //     Vector4::new(),
        // ])
    }

    pub fn as_fps_view_matrix(&self) -> Matrix4<f32> {
        let (_, pitch, yaw) = self.rotation.euler_angles();

        let cos_pitch = pitch.cos();
        let sin_pitch = pitch.sin();
        let cos_yaw = yaw.cos();
        let sin_yaw = yaw.sin();

        let eye = &self.position;

        let xaxis = Vector3::new(cos_yaw, 0.0, -sin_yaw);
        let yaxis = Vector3::new(sin_yaw * sin_pitch, cos_pitch, cos_yaw * sin_pitch);
        let zaxis = Vector3::new(sin_yaw * cos_pitch, -sin_pitch, cos_pitch * cos_yaw);

        Matrix4::from_columns(&[
            Vector4::new(xaxis.x, yaxis.x, zaxis.x, 0.0),
            Vector4::new(xaxis.y, yaxis.y, zaxis.y, 0.0),
            Vector4::new(xaxis.z, yaxis.z, zaxis.z, 0.0),
            Vector4::new(-xaxis.dot(eye), -yaxis.dot(eye), -zaxis.dot(eye), 1.0),
        ])
    }

    pub fn translate(&mut self, t: Vector3<f32>) {
        // if t != zero() {
            self.position += self.rotation * t;
        // }
    }

    pub fn translate_along(&mut self, dir: Vector3<f32>, scaler: f32) {
        if dir != zero() {
            self.position += self.rotation * { dir.normalize() * scaler };
        }
    }

    pub fn translate_forward(&mut self, s: f32) {
        self.translate(Vector3::new(0.0, 0.0, -s))
    }

    pub fn translate_right(&mut self, s: f32) {
        self.translate(Vector3::new(s, 0.0, 0.0))
    }

    pub fn rotate_global(&mut self, r: UnitQuaternion<f32>) {
        // if r != UnitQuaternion::identity() {
            self.rotation = r * self.rotation;
        // }
    }

    pub fn rotate_local(&mut self, r: UnitQuaternion<f32>) {
        // if r != UnitQuaternion::identity() {
            self.rotation = self.rotation * r;
        // }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            position: Vector3::new(0.0, 0.0, 0.0),
            // rotation: UnitQuaternion::look_at_rh(&-Vector3::z(), &-Vector3::y()),
            rotation: UnitQuaternion::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
