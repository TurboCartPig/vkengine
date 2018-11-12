use components::Transform;
use float_duration::TimePoint;
use na::{UnitQuaternion, Vector3};
use renderer::camera::ActiveCamera;
use resources::{DeltaTime, Keyboard, Mouse};
use specs::prelude::*;
use std::{mem, time::Instant};
use winit::VirtualKeyCode;
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::PI;

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
    type SystemData = Write<'a, DeltaTime>;

    fn run(&mut self, mut delta_time: Self::SystemData) {
        let now = Instant::now();

        let delta = now.float_duration_since(self.last_frame).unwrap();
        delta_time.delta = delta.as_seconds();

        let first_frame = now.float_duration_since(self.first_frame).unwrap();
        delta_time.first_frame = first_frame.as_seconds();

        mem::replace(&mut self.last_frame, now);
    }
}

pub struct TransformSystem{
    pitch: f32,
    yaw: f32,
}

impl Default for TransformSystem {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

impl<'a> System<'a> for TransformSystem {
    type SystemData = (
        Read<'a, Keyboard>,
        Write<'a, Mouse>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (keyboard, mut mouse, delta_time, active_camera, mut transform): Self::SystemData,
    ) {
        // If mouse is not grabbed, then the window is not focused, and we sould not handle input
        if !mouse.grabbed {
            return;
        }

        let (_, camera_t) = (&active_camera, &mut transform).join().next().unwrap();

        if keyboard.pressed(VirtualKeyCode::W) {
            camera_t.translate_forward(1.0 * delta_time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::S) {
            camera_t.translate_forward(-1.0 * delta_time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::A) {
            camera_t.translate_right(-1.0 * delta_time.delta as f32);
        }

        if keyboard.pressed(VirtualKeyCode::D) {
            camera_t.translate_right(1.0 * delta_time.delta as f32);
        }

        let (yaw, pitch) = mouse.move_delta;
        let (yaw, pitch) = (yaw as f32 * 0.001, pitch as f32 * 0.001);

        self.pitch += pitch;
        self.yaw += yaw;

        if self.pitch > FRAC_PI_2 { self.pitch = FRAC_PI_2 }
        if self.pitch < -FRAC_PI_2 { self.pitch = -FRAC_PI_2 }
        if self.yaw > PI * 2.0 { self.yaw -= PI * 2.0 }
        if self.yaw < -PI * 2.0 { self.yaw += PI * 2.0 }

        camera_t.rotation = UnitQuaternion::from_euler_angles(0.0, self.pitch, self.yaw);

        // camera_t.rotate_local(UnitQuaternion::from_axis_angle(&Vector3::x_axis(), pitch));
        // camera_t.rotate_global(UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw));
        // camera_t.rotate_local(UnitQuaternion::from_scaled_axis(Vector3::x() * pitch));
        // camera_t.rotate_global(UnitQuaternion::from_scaled_axis(Vector3::y() * yaw));

        *mouse = Mouse::default();
    }
}

#[allow(dead_code)]
pub struct PrintSystem {
    counter: u32,
}

impl<'a> System<'a> for PrintSystem {
    type SystemData = ReadStorage<'a, Transform>;

    fn run(&mut self, transform: Self::SystemData) {
        let freq = 60;
        if self.counter == freq {
            for t in transform.join() {
                println!("Hello transform {:?}", t);
            }
            self.counter = 0;
        } else {
            self.counter += 1;
        }
    }
}

impl Default for PrintSystem {
    fn default() -> Self {
        Self { counter: 0 }
    }
}
