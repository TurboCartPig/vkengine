use components::{DeltaTime, Transform, Keyboard};
use renderer::camera::Camera;
use float_duration::TimePoint;
use na::Vector3;
use na::Rotation3;
use na::Unit;
use specs::prelude::*;
use winit::VirtualKeyCode;
use std::{mem, time::Instant};

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

pub struct TransformSystem;

impl<'a> System<'a> for TransformSystem {
    type SystemData = (Read<'a, Keyboard>,
                       Read<'a, DeltaTime>,
                       ReadStorage<'a, Camera>,
                       WriteStorage<'a, Transform>);

    fn run(&mut self, (keyboard, delta_time, camera, mut transform): Self::SystemData) {
        let (_, camera_t) = (&camera, &mut transform).join().next().unwrap();

        if keyboard.pressed(VirtualKeyCode::W) {
            camera_t.position = Vector3::new(camera_t.position.x, camera_t.position.y, camera_t.position.z + 1.0 * delta_time.delta as f32);
        }
        
        let forward = Rotation3::from_euler_angles(camera_t.rotation.0, camera_t.rotation.1, camera_t.rotation.2).scaled_axis();
        println!("Forward: {:?}", forward);

    }
}

pub struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    type SystemData = ReadStorage<'a, Transform>;

    fn run(&mut self, transform: Self::SystemData) {
        for t in transform.join() {
            println!("Hello transform {:?}", t);
        }
    }
}
