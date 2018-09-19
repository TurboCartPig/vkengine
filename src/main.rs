extern crate winit;
#[macro_use]
extern crate vulkano;
extern crate vulkano_win;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate alga;
extern crate float_duration;
extern crate genmesh;
extern crate nalgebra as na;
extern crate specs;
#[macro_use]
extern crate specs_derive;

mod components;
mod renderer;
mod systems;

use self::{
    components::{DeltaTime, ShouldClose, Transform},
    renderer::{
        geometry::{Shape, ShapedMeshBuilder},
        Renderer,
    },
    systems::{PrintSystem, TimeSystem},
};
use na::Vector3;
use specs::prelude::*;
use winit::EventsLoop;

//TODO Use a logger instead of println
//TODO Mesh loading
//TODO Use glyph-brush insted of vulkano_text
//TODO Fix/Impl lighting

struct EventsLoopSystem {
    events_loop: EventsLoop,
}

impl EventsLoopSystem {
    pub fn new(events_loop: EventsLoop) -> Self {
        EventsLoopSystem { events_loop }
    }
}

impl<'a> System<'a> for EventsLoopSystem {
    type SystemData = Write<'a, ShouldClose>;

    fn run(&mut self, mut should_close: Self::SystemData) {
        // Event handeling
        self.events_loop.poll_events(|event| {
            use winit::{
                DeviceEvent,
                Event::{DeviceEvent as Device, WindowEvent as Window},
                KeyboardInput, WindowEvent,
            };

            match event {
                Window {
                    event: WindowEvent::CloseRequested,
                    ..
                } => should_close.0 = true,
                Device {
                    event: DeviceEvent::Key(KeyboardInput { scancode: 0x10, .. }),
                    ..
                } => should_close.0 = true,
                _ => (),
            }
        });
    }
}

fn main() {
    // The wayland backend for winit is in a pretty poor state as of now, so we use x11 instead
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    let events_loop = EventsLoop::new();
    let renderer = Renderer::new(&events_loop);
    let events_loop_system = EventsLoopSystem::new(events_loop);

    // ECS World
    let mut world = World::new();

    // Register components
    world.register::<Transform>();
    world.register::<renderer::geometry::MeshComponent>();

    // Add resources
    world.add_resource(DeltaTime::default());
    world.add_resource(ShouldClose::default());

    // Create entities
    world.create_entity().with(Transform::default()).build();
    let t = Transform {
        position: Vector3::new(0.0, 0.0, -3.0),
        rotation: (0.0, 90.0, 0.0),
        scale: Vector3::new(1.0, 1.0, 1.0),
    };
    world
        .create_entity()
        .with(t)
        .with(ShapedMeshBuilder::new(
            renderer.device.clone(),
            Shape::Plane(None),
        )).build();
    world
        .create_entity()
        .with(Transform {
            position: Vector3::new(2.0, 0.0, -5.0),
            ..Transform::default()
        }).with(ShapedMeshBuilder::new(renderer.device.clone(), Shape::Cube))
        .build();

    // Create dispatcher
    let mut dispatcher = DispatcherBuilder::new()
        //.with(PrintSystem, "print", &[])
        .with(TimeSystem::default(), "time", &[])
        .with(renderer, "renderer", &["time"])
        .with_barrier()
        .with_thread_local(events_loop_system)
        .build();

    // Setup the systems
    dispatcher.setup(&mut world.res);

    // The gameloop dispatches the systems and checks if the game should close
    'gameloop: loop {
        dispatcher.dispatch(&world.res);
        world.maintain();

        if world.read_resource::<ShouldClose>().0 {
            break 'gameloop;
        }
    }
}
