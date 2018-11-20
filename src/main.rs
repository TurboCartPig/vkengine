#![feature(custom_attribute)]

mod components;
mod renderer;
mod resources;
mod systems;

use crate::{
    components::Transform,
    renderer::{
        camera::{ActiveCamera, Camera},
        geometry::{MeshComponent, Shape},
        Renderer, Surface,
    },
    resources::{DeltaTime, Keyboard, Mouse, ShouldClose},
    systems::{TimeSystem, TransformSystem},
};
use specs::prelude::*;
use winit::EventsLoop;

//TODO Use a logger instead of println
//TODO Mesh loading
//TODO Use glyph-brush insted of vulkano_text
//TODO Fix/Impl lighting
//TODO Fix all tranformations and matrix math

struct EventsLoopSystem {
    events_loop: EventsLoop,
    surface: Surface,
}

impl EventsLoopSystem {
    pub fn new(events_loop: EventsLoop, surface: Surface) -> Self {
        EventsLoopSystem {
            events_loop,
            surface,
        }
    }
}

impl<'a> System<'a> for EventsLoopSystem {
    type SystemData = (
        Write<'a, ShouldClose>,
        Write<'a, Keyboard>,
        Write<'a, Mouse>,
    );

    fn run(&mut self, (mut should_close, mut keyboard, mut mouse): Self::SystemData) {
        let window = self.surface.window();
        // Event handeling
        self.events_loop.poll_events(|event| {
            use winit::{
                DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta,
                WindowEvent,
            };

            match event {
                Event::WindowEvent {
                    window_id: _,
                    event,
                } => match event {
                    WindowEvent::CloseRequested => should_close.0 = true,
                    WindowEvent::Destroyed => should_close.0 = true,
                    WindowEvent::Focused(grabbed) => {
                        mouse.grabbed = grabbed;
                        window.grab_cursor(grabbed).unwrap();
                        window.hide_cursor(grabbed);
                    }
                    _ => (),
                },
                Event::DeviceEvent {
                    device_id: _,
                    event,
                } => match event {
                    DeviceEvent::MouseMotion { delta } => {
                        mouse.move_delta.0 += delta.0;
                        mouse.move_delta.1 += delta.1;
                    }
                    DeviceEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(x, y),
                    } => mouse.scroll_delta = (x, y),
                    DeviceEvent::Key(KeyboardInput {
                        state,
                        virtual_keycode: Some(key),
                        ..
                    }) => {
                        match state {
                            ElementState::Pressed => keyboard.press(key),
                            ElementState::Released => keyboard.release(key),
                        };
                    }
                    _ => (),
                },
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
    let events_loop_system = EventsLoopSystem::new(events_loop, renderer.surface());

    // ECS World
    let mut world = World::new();

    // Register components
    world.register::<Transform>();
    world.register::<MeshComponent>();
    world.register::<ActiveCamera>();
    world.register::<Camera>();

    // Add resources
    world.add_resource(DeltaTime::default());
    world.add_resource(ShouldClose::default());
    world.add_resource(Keyboard::default());
    world.add_resource(Mouse::default());

    // Create entities
    world.create_entity().with(Transform::default()).build();

    // Plane
    // world
    //     .create_entity()
    //     .with(Transform {
    //         position: Vector3::new(0.0, 0.0, -3.0),
    //         rotation: UnitQuaternion::from_euler_angles(0.0, std::f32::consts::FRAC_PI_4, 0.0),
    //         ..Transform::default()
    //     })
    //     .with(MeshComponent::from_shape(
    //         renderer.device.clone(),
    //         Shape::Plane(None),
    //     ))
    //     .build();

    // Cube
    world
        .create_entity()
        .with(Transform::default())
        .with(MeshComponent::from_shape(
            renderer.device.clone(),
            Shape::Cube,
        ))
        .build();

    // Camera
    world
        .create_entity()
        .with(Transform::default())
        .with(Camera::default())
        .with(ActiveCamera)
        .build();

    // Create dispatcher
    let mut dispatcher = DispatcherBuilder::new()
        // .with(PrintSystem::default(), "print", &[])
        .with(TimeSystem::default(), "time", &[])
        .with(TransformSystem, "transform", &["time"])
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
