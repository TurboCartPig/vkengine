use nalgebra::{zero, Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3};
use specs::prelude::*;
use std::ops::{AddAssign, Deref, DerefMut};

/// A Wrapper around the local and the global transform
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlobalTransform {
    pub global: Transform,
}

impl Component for GlobalTransform {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl Deref for GlobalTransform {
    type Target = Transform;

    fn deref(&self) -> &Self::Target {
        &self.global
    }
}

impl DerefMut for GlobalTransform {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.global
    }
}

impl From<Transform> for GlobalTransform {
    fn from(global: Transform) -> Self {
        Self { global }
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
    pub fn from_parts(translation: Vector3<f32>, quat: UnitQuaternion<f32>, scale: Vector3<f32>) -> Self {
        Self {
            iso: Isometry3::from_parts(Translation3::from(translation), quat),
            scale,
        }
    }

    pub fn to_matrix(&self) -> Matrix4<f32> {
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

    pub fn translation(&self) -> &Vector3<f32> {
        &self.iso.translation.vector
    }

    pub fn rotation(&self) -> &UnitQuaternion<f32> {
        &self.iso.rotation
    }

    pub fn scale(&self) -> &Vector3<f32> {
        &self.scale
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

impl AddAssign<Transform> for Transform {
    fn add_assign(&mut self, other: Transform) {
        self.iso.translation.vector += other.iso.translation.vector;
        self.iso.rotation *= other.iso.rotation;
        self.scale = Vector3::new(
            self.scale.x * other.scale.x,
            self.scale.y * other.scale.y,
            self.scale.z * other.scale.z,
        );
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
