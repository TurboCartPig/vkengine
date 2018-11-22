mod transform;

pub use crate::systems::transform::TransformSystem;

use crate::{
    components::Transform,
    renderer::camera::ActiveCamera,
    resources::{Keyboard, Mouse, Time},
};
use float_duration::TimePoint;
use nalgebra::{UnitQuaternion, Vector3};
use specs::prelude::*;
use std::{mem, time::Instant};
use winit::VirtualKeyCode;

/// A System for updating the Time resource in order to expose things like delta time
pub struct TimeSystem {
    first_frame: Instant,
    last_frame: Instant,
}

impl Default for TimeSystem {
    fn default() -> Self {
        TimeSystem {
            first_frame: Instant::now(),
            last_frame: Instant::now(),
        }
    }
}

impl<'a> System<'a> for TimeSystem {
    type SystemData = Write<'a, Time>;

    fn run(&mut self, mut time: Self::SystemData) {
        let now = Instant::now();

        let delta = now.float_duration_since(self.last_frame).unwrap();
        time.delta = delta.as_seconds();

        let first_frame = now.float_duration_since(self.first_frame).unwrap();
        time.first_frame = first_frame.as_seconds();

        mem::replace(&mut self.last_frame, now);
    }
}

/// Fly control system
pub struct FlyControlSystem;

impl<'a> System<'a> for FlyControlSystem {
    type SystemData = (
        Read<'a, Keyboard>,
        Write<'a, Mouse>,
        Read<'a, Time>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(&mut self, (keyboard, mut mouse, time, active_camera, mut transform): Self::SystemData) {
        // If mouse is not grabbed, then the window is not focused, and we sould not handle input
        if !mouse.grabbed {
            return;
        }

        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        // Rotation
        let (yaw, pitch) = mouse.move_delta;
        let (yaw, pitch) = (yaw as f32 * -0.001, pitch as f32 * -0.001);

        camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        // Reset mouse input
        mouse.clear_deltas();

        // Translation
        if keyboard.pressed(VirtualKeyCode::W) {
            camera_t.translate_forward(1.0 * time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::S) {
            camera_t.translate_forward(-1.0 * time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::A) {
            camera_t.translate_right(-1.0 * time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::D) {
            camera_t.translate_right(1.0 * time.delta as f32);
        }
    }
}
