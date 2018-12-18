use nalgebra::{zero, Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3};
use specs::prelude::*;
use specs_derive::Component;

/// Absolute transform matrix
#[derive(Component, Clone, Debug, PartialEq)]
#[storage(VecStorage)]
pub struct TransformMatrix {
    pub mat: Matrix4<f32>,
}

impl From<Matrix4<f32>> for TransformMatrix {
    fn from(mat: Matrix4<f32>) -> Self {
        Self { mat }
    }
}

impl Default for TransformMatrix {
    fn default() -> Self {
        Self {
            mat: Matrix4::identity(),
        }
    }
}

/// Transform (translation, rotation, scale)
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    // Isometry: Translation and rotation
    pub iso: Isometry3<f32>,
    // Nonuniform scale
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn to_matrix(&self) -> Matrix4<f32> {
        // Matrix4::new_nonuniform_scaling(&self.scale)
        //     * Matrix4::new_rotation(self.rotation.scaled_axis())
        //     * Matrix4::new_translation(&self.position)

        self.iso
            .to_homogeneous()
            .prepend_nonuniform_scaling(&self.scale)
    }

    pub fn to_view_matrix(&self) -> Matrix4<f32> {
        let inverse_scale =
            Vector3::new(1.0 / self.scale.x, 1.0 / self.scale.y, 1.0 / self.scale.z);
        self.iso
            .inverse()
            .to_homogeneous()
            .append_nonuniform_scaling(&inverse_scale)
    }

    pub fn translate(&mut self, t: Vector3<f32>) {
        if t != zero() {
            self.iso.translation.vector += self.iso.rotation * t;
        }
    }

    pub fn translate_along(&mut self, dir: Vector3<f32>, scaler: f32) {
        if dir != zero() {
            self.iso.translation.vector += self.iso.rotation * { dir.normalize() * scaler };
        }
    }

    pub fn translate_forward(&mut self, scaler: f32) {
        self.translate(Vector3::new(0.0, 0.0, -scaler))
    }

    pub fn translate_right(&mut self, scaler: f32) {
        self.translate(Vector3::new(scaler, 0.0, 0.0))
    }

    pub fn rotate_global(&mut self, r: UnitQuaternion<f32>) {
        self.iso.rotation = r * self.iso.rotation;
    }

    pub fn rotate_local(&mut self, r: UnitQuaternion<f32>) {
        self.iso.rotation *= r;
    }
}

impl Component for Transform {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Into<Matrix4<f32>> for Transform {
    fn into(self) -> Matrix4<f32> {
        self.to_matrix()
    }
}

impl From<Vector3<f32>> for Transform {
    fn from(vector: Vector3<f32>) -> Self {
        let mut iso = Isometry3::identity();
        iso.append_translation_mut(&Translation3::from(vector));
        Self {
            iso,
            ..Self::default()
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform {
            iso: Isometry3::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
