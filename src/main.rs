extern crate winit;
#[macro_use]
extern crate vulkano;
extern crate vulkano_win;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate float_duration;
extern crate genmesh;
extern crate nalgebra as na;
extern crate specs;
#[macro_use]
extern crate specs_derive;

mod components;
mod renderer;

use self::components::Transform;
use self::renderer::geometry::{MeshComponent, Shape, ShapedMeshBuilder};
use float_duration::{FloatDuration, TimePoint};
use na::Vector3;
use specs::prelude::*;
use std::time::Instant;
use vulkano::sync;

//TODO Use a logger instead of println
//TODO Mesh loading
//TODO Use glyph-brush insted of vulkano_text

struct PrintSystem;

impl<'a> System<'a> for PrintSystem {
    type SystemData = ReadStorage<'a, Transform>;

    fn run(&mut self, transform: Self::SystemData) {
        for t in transform.join() {
            println!("Hello transform {:?}", t);
        }
    }
}

fn main() {
    // The wayland backend for winit is in a pretty poor state as of now, so we use x11 instead
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let mut events_loop = winit::EventsLoop::new();
    let mut renderer = renderer::Renderer::new(&events_loop);

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<renderer::geometry::MeshComponent>();

    world.create_entity().with(Transform::default()).build();
    let t = Transform {
        position: Vector3::new(0.0, 0.0, -3.0),
        rotation: (20.0, 20.0, 20.0),
        ..Transform::default()
    };
    world
        .create_entity()
        .with(t)
        .with(ShapedMeshBuilder::new(renderer.device.clone(), Shape::Cube))
        .build();

    let mut print = PrintSystem;
    print.run_now(&world.res);
    world.maintain();

    let mut previous_frame_end =
        Box::new(sync::now(renderer.device.clone())) as Box<sync::GpuFuture>;
    let mut frame_time = FloatDuration::seconds(1f64); // Cant ever be zero
    let mut should_close = false;

    'gameloop: loop {
        let frame_start_time = Instant::now();

        // Render and draw the next frame
        //previous_frame_end = renderer.render(previous_frame_end, frame_time);

        renderer.run_now(&world.res);
        world.maintain();

        // Event handeling
        events_loop.poll_events(|event| {
            use winit::Event::DeviceEvent as Device;
            use winit::Event::WindowEvent as Window;
            use winit::{DeviceEvent, KeyboardInput, WindowEvent};

            match event {
                Window {
                    event: WindowEvent::CloseRequested,
                    ..
                } => should_close = true,
                Device {
                    event: DeviceEvent::Key(KeyboardInput { scancode: 0x10, .. }),
                    ..
                } => should_close = true,
                _ => (),
            }
        });

        if should_close {
            break 'gameloop;
        }

        // Find frametime
        let frame_end_time = Instant::now();
        frame_time = frame_end_time
            .float_duration_since(frame_start_time)
            .unwrap_or(frame_time);
    }
}
