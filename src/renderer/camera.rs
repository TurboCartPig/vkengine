use na::{Matrix4, Perspective3, Point3, Vector3};
use specs::HashMapStorage;

static CLIP_NEAR: f32 = 0.01f32;
static CLIP_FAR: f32 = 100f32;

#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Camera {
    pub projection: Perspective3<f32>,
    pub view: Matrix4<f32>,
    pub scale: Matrix4<f32>,
    fovy: f32,
}

impl Camera {
    pub fn new(aspect: f32, fovy: f32) -> Self {
        let projection = Perspective3::new(aspect, fovy, CLIP_NEAR, CLIP_FAR);

        let view = Matrix4::look_at_rh(
            &Point3::new(0.0, 0.0, 1.0),
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, -1.0, 0.0),
        );

        let scale = Matrix4::new_scaling(1.0);

        Self {
            projection,
            view,
            scale,
            fovy,
        }
    }

    pub fn update_aspect(&mut self, aspect: f32) {
        self.projection = Perspective3::new(aspect, self.fovy, CLIP_NEAR, CLIP_FAR);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new({ 16 / 9 } as f32, { 3.14 / 2. } as f32)
    }
}
