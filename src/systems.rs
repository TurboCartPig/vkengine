use components::{DeltaTime, Transform};
use float_duration::TimePoint;
use specs::prelude::*;
use std::mem;
use std::time::Instant;

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

pub struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    type SystemData = ReadStorage<'a, Transform>;

    fn run(&mut self, transform: Self::SystemData) {
        for t in transform.join() {
            println!("Hello transform {:?}", t);
        }
    }
}
