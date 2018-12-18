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
        RenderEvents, Renderer,
    },
    resources::{FocusGained, KeyboardEvents, ShouldClose, Time},
    systems::{
        FlyControlSystem, GameInput, GameInputSystem, SDLSystem, TimeSystem, TransformSystem,
    },
};
use nalgebra::Vector3;
use specs::prelude::*;
use specs_hierarchy::HierarchySystem;

//TODO Mesh loading
//TODO Use glyph-brush insted of vulkano_text
//TODO Fix/Impl lighting

fn main() {
    env_logger::init();

    let sdl = SDLSystem::new();
    let renderer = Renderer::new(sdl.window());

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
    world.add_resource(FocusGained::default());
    world.add_resource(GameInput::default());
    world.add_resource(RenderEvents::default());
    world.add_resource(KeyboardEvents::default());

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
        .with(GameInputSystem::default(), "input", &[])
        .with(FlyControlSystem, "fly", &["time"])
        .with(renderer, "renderer", &["time", "transform", "fly"])
        .with_barrier()
        .with_thread_local(sdl)
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
