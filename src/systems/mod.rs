mod transform;

pub use crate::systems::transform::TransformSystem;

use crate::{
    components::Transform,
    renderer::camera::ActiveCamera,
    resources::{FocusGained, Keyboard, Keycode, Mouse, MouseButton, Time},
};
use float_duration::TimePoint;
use nalgebra::{abs, UnitQuaternion, Vector3};
use specs::prelude::*;
use std::{mem, time::Instant};

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
        Read<'a, Time>,
        Read<'a, FocusGained>,
        Read<'a, Keyboard>,
        Read<'a, Mouse>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (time, input_enabled, keyboard, mouse, active_camera, mut transform): Self::SystemData,
    ) {
        // Only handle input if the window is focused
        // ------------------------------------------------------------------------------------------------------------
        if !input_enabled.0 {
            return;
        }

        // Get the camera transform
        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        // Rotation
        // ------------------------------------------------------------------------------------------------------------
        let yaw = -mouse.delta_x() as f32;
        let pitch = -mouse.delta_y() as f32;
        // Input scaling
        let (yaw, pitch) = (yaw * 0.001, pitch * 0.001);

        camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        // Translation
        // ------------------------------------------------------------------------------------------------------------
        if keyboard.pressed(Keycode::W) {
            camera_t.translate_forward(1.0 * time.delta as f32);
        }

        if keyboard.pressed(Keycode::S) {
            camera_t.translate_forward(-1.0 * time.delta as f32);
        }

        if keyboard.pressed(Keycode::A) {
            camera_t.translate_right(-1.0 * time.delta as f32);
        }

        if keyboard.pressed(Keycode::D) {
            camera_t.translate_right(1.0 * time.delta as f32);
        }
    }
}
