use na::{Matrix4, Orthographic3, Perspective3, Point3, Vector3};

static CLIP_NEAR: f32 = 0.01f32;
static CLIP_FAR: f32 = 100f32;

pub struct Camera<P> {
    pub projection: P,
    pub view: Matrix4<f32>,
    pub scale: Matrix4<f32>,
    fovy: f32,
}

impl Camera<Perspective3<f32>> {
    pub fn new(aspect: f32, fovy: f32) -> Self {
        let projection = Perspective3::new(aspect, fovy, CLIP_NEAR, CLIP_FAR);

        let view = Matrix4::look_at_rh(
            &Point3::new(0.3, 0.3, 1.0),
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

impl Camera<Orthographic3<f32>> {
    // FIXME The values are all just random and this has not been tested
    pub fn new() -> Self {
        let projection = Orthographic3::new(-50.0, 50.0, -50.0, 50.0, CLIP_NEAR, CLIP_FAR);

        let view = Matrix4::look_at_rh(
            &Point3::new(0.0, 0.0, 0.0),
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, -1.0, 0.0),
        );

        let scale = Matrix4::new_scaling(1.0);

        Self {
            projection,
            view,
            scale,
            fovy: 0.0,
        }
    }
}
