use nalgebra::{Matrix4, Perspective3};
use specs::{Component, HashMapStorage, NullStorage};
use specs_derive::Component;

static CLIP_NEAR: f32 = 0.01f32;
static CLIP_FAR: f32 = 100f32;

#[derive(Component, Default)]
#[storage(NullStorage)]
pub struct ActiveCamera;

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Camera {
    pub projection: Perspective3<f32>,
    pub scale: Matrix4<f32>,
    fovy: f32,
}

impl Camera {
    pub fn new(aspect: f32, fovy: f32) -> Self {
        let projection = Perspective3::new(aspect, fovy, CLIP_NEAR, CLIP_FAR);

        let scale = Matrix4::new_scaling(1.0);

        Self {
            projection,
            scale,
            fovy,
        }
    }

    pub fn update_aspect(&mut self, aspect: f32) {
        self.projection = Perspective3::new(aspect, self.fovy, CLIP_NEAR, CLIP_FAR);
    }

    pub fn projection(&self) -> [[f32; 4]; 4] {
        let mut p: [[f32; 4]; 4] = self.projection.unwrap().into();

        // Flip the y-axis
        p[1][1] *= -1.0;

        p
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(16. / 9., std::f32::consts::FRAC_PI_2)
    }
}
