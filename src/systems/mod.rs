mod transform;

pub use crate::systems::transform::TransformSystem;

use crate::{
    components::Transform,
    renderer::camera::ActiveCamera,
    resources::{Gamepad, Keyboard, Mouse, Time},
};
use float_duration::TimePoint;
use input::platform::xinput::{Device, DeviceState};
use nalgebra::{abs, UnitQuaternion, Vector3};
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
        // Read<'a, Gamepad>,
        Read<'a, Device>,
        Read<'a, Time>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (keyboard, mut mouse, gamepad, time, active_camera, mut transform): Self::SystemData,
    ) {
        // If mouse is not grabbed, then the window is not focused, and we sould not handle input
        if !mouse.grabbed {
            return;
        }

        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        // Rotation
        let state = gamepad.get_state().unwrap();
        let (yaw, pitch) = (-state.stick_right_x, state.stick_right_y);
        // let dir = gamepad.right.to_vector();

        // if dir.magnitude() > 0.1 {
        //     yaw = -dir.x * abs(&gamepad.right.x.value());
        //     pitch = dir.y * abs(&gamepad.right.y.value());
        // } else {
        //     yaw = -mouse.move_delta.0 as f32;
        //     pitch = -mouse.move_delta.1 as f32;
        // }

        let (yaw, pitch) = (yaw * 0.001, pitch * 0.001);

        camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        // Reset mouse input
        mouse.clear_deltas();

        {
            use input::platform::{key_down, VirtualKeyCode};
            let eh = key_down(VirtualKeyCode::Q);
            if eh {
                println!("Q is: {}", eh);
            }
        }
        {
            use input::platform::xinput::XINPUT_GAMEPAD_LEFT_THUMB;
            if state.buttons & XINPUT_GAMEPAD_LEFT_THUMB == XINPUT_GAMEPAD_LEFT_THUMB {
                println!("Left thumb is down");
            }
        }

        // Translation
        // if gamepad.left.to_vector().magnitude() > 0.1 {
        //     let dir = gamepad.left.to_vector();

        //     let dir = Vector3::new(dir.x, 0., -dir.y);

        //     camera_t.translate_along(dir, time.delta as f32);
        if true {
            camera_t.translate_forward(state.stick_left_y * time.delta as f32);
            camera_t.translate_right(state.stick_left_x * time.delta as f32);
        } else {
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
}
