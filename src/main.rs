#![feature(custom_attribute)]

mod components;
mod renderer;
mod resources;
mod systems;

use crate::{
    components::{Link, Transform, TransformMatrix},
    renderer::{
        camera::{ActiveCamera, Camera},
        geometry::{MeshComponent, Shape},
        Renderer, Surface,
    },
    resources::{Gamepad, Keyboard, Mouse, ShouldClose, Time},
    systems::{FlyControlSystem, TimeSystem, TransformSystem},
};
use gilrs::Gilrs;
use log::{info, warn};
use nalgebra::Vector3;
use specs::prelude::*;
use specs_hierarchy::HierarchySystem;
use winit::EventsLoop;
#[cfg(target_os = "windows")]
use input::platform::xinput::Device;

//TODO Mesh loading
//TODO Use glyph-brush insted of vulkano_text
//TODO Fix/Impl lighting

/// Turns raw events from the os into data in the ecs world
struct RawEventSystem {
    events_loop: EventsLoop,
    surface: Surface,
    gilrs: Gilrs,
}

impl RawEventSystem {
    pub fn new(events_loop: EventsLoop, surface: Surface) -> Self {
        RawEventSystem {
            events_loop,
            surface,
            gilrs: Gilrs::new().expect("Failed to create gilrs object"),
        }
    }
}

impl<'a> System<'a> for RawEventSystem {
    type SystemData = (
        Write<'a, ShouldClose>,
        Write<'a, Keyboard>,
        Write<'a, Mouse>,
        Write<'a, Gamepad>,
    );

    fn run(&mut self, (mut should_close, mut keyboard, mut mouse, mut gamepad): Self::SystemData) {
        let window = self.surface.window();

        // Winit event handeling
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

        // Gilrs event handeling
        {
            use gilrs::{Axis, Event, EventType};

            while let Some(event) = self.gilrs.next_event() {
                match event {
                    Event {
                        event: EventType::Disconnected,
                        ..
                    } => {
                        warn!("Gamepad disconnected");
                    }
                    Event {
                        event: EventType::Connected,
                        ..
                    } => {
                        info!("Gamepad connected");
                    }
                    Event {
                        event: EventType::ButtonPressed(button, _),
                        ..
                    } => {
                        gamepad.press(button);
                    }
                    Event {
                        event: EventType::ButtonReleased(button, _),
                        ..
                    } => {
                        gamepad.release(button);
                    }
                    Event {
                        event: EventType::AxisChanged(axis, delta, _),
                        ..
                    } => match axis {
                        Axis::LeftStickX => gamepad.left.x.delta(delta),
                        Axis::LeftStickY => gamepad.left.y.delta(delta),
                        Axis::RightStickX => gamepad.right.x.delta(delta),
                        Axis::RightStickY => gamepad.right.y.delta(delta),
                        Axis::LeftZ => gamepad.lbumper.delta(delta),
                        Axis::RightZ => gamepad.rbumper.delta(delta),
                        Axis::DPadX => {}
                        Axis::DPadY => {}
                        Axis::Unknown => {}
                    },
                    _ => (),
                }
            }

            self.gilrs.inc();
        }
    }
}

fn main() {
    // The wayland backend for winit is in a pretty poor state as of now, so we use x11 instead
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

    env_logger::init();
    let events_loop = EventsLoop::new();
    let renderer = Renderer::new(&events_loop);
    let events_loop_system = RawEventSystem::new(events_loop, renderer.surface());

    // ECS World
    let mut world = World::new();

    // Register components
    world.register::<Link>();
    world.register::<Transform>();
    world.register::<TransformMatrix>();
    world.register::<MeshComponent>();
    world.register::<ActiveCamera>();
    world.register::<Camera>();

    // Add resources
    world.add_resource(Time::default());
    world.add_resource(ShouldClose::default());
    world.add_resource(Keyboard::default());
    world.add_resource(Mouse::default());
    world.add_resource(Gamepad::default());
    world.add_resource(Device::new(0, None));

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

    let parent = world
        .create_entity()
        .with(Transform::from(Vector3::new(1.0, 0.0, -10.0)))
        .build();
    // Cube
    world
        .create_entity()
        .with(Link::new(parent))
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
        .with(HierarchySystem::<Link>::new(), "hierarchy", &[])
        .with(TransformSystem::default(), "transform", &["hierarchy"])
        .with(FlyControlSystem, "fly", &["time"])
        .with(renderer, "renderer", &["time", "transform", "fly"])
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
