mod components;
mod renderer;
mod resources;
mod systems;

use crate::{
    components::{GlobalTransform, Link, Transform},
    renderer::{
        camera::{ActiveCamera, Camera},
        geometry::{MeshBuilder, MeshComponent, Shape},
        lights::{DirectionalLightRes, PointLightComponent},
        RenderEvents, Renderer,
    },
    resources::{DirtyEntities, FocusGained, KeyboardEvents, ShouldClose, Time},
    systems::{
        FlyControlSystem, GameInput, GameInputSystem, PlacerSystem, SDLSystem, TimeSystem,
        TransformSystem,
    },
};
use nalgebra::Vector3;
use specs::prelude::*;
use specs_hierarchy::HierarchySystem;

//TODO Mesh loading
//TODO Use glyph-brush for text
//TODO Use Warmy for resource loading
//TODO Serialize scenes from file

fn main() {
    env_logger::init();

    let sdl = SDLSystem::new();
    let renderer = Renderer::new(sdl.window());

    // ECS World
    let mut world = World::new();

    // Register components
    world.register::<Link>();
    world.register::<Transform>();
    world.register::<GlobalTransform>();
    world.register::<MeshComponent>();
    world.register::<MeshBuilder>();
    world.register::<ActiveCamera>();
    world.register::<Camera>();
    world.register::<PointLightComponent>();

    // Add resources
    world.add_resource(Time::default());
    world.add_resource(ShouldClose::default());
    world.add_resource(FocusGained::default());
    world.add_resource(GameInput::default());
    world.add_resource(RenderEvents::default());
    world.add_resource(KeyboardEvents::default());
    world.add_resource(DirectionalLightRes::default());
    world.add_resource(DirtyEntities::default());

    // Create entities
    world.create_entity().with(Transform::default()).build();

    let parent = world
        .create_entity()
        .with(Transform::from(Vector3::new(1.0, 0.0, -10.0)))
        .build();

    // Sphere
    world
        .create_entity()
        .with(Link::new(parent))
        .with(Transform::default())
        .with(MeshBuilder::new().with_shape(Shape::Sphere(100, 100)))
        .build();

    // Cylinder
    world
        .create_entity()
        .with(Transform::from(Vector3::new(5.0, 1.0, -7.0)))
        .with(MeshBuilder::new().with_shape(Shape::Cylinder(40)))
        .with(PointLightComponent::from_color(Vector3::new(0.0, 0.0, 1.0)))
        .build();

    // Cube
    world
        .create_entity()
        .with(Transform::from(Vector3::new(-2.0, -4.0, 5.0)))
        .with(MeshBuilder::new().with_shape(Shape::Cube))
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
        .with(TimeSystem::default(), "time", &[])
        .with(HierarchySystem::<Link>::new(), "hierarchy", &[])
        .with(TransformSystem::default(), "transform", &["hierarchy"])
        .with(GameInputSystem::default(), "input", &[])
        .with(FlyControlSystem, "fly", &["time", "input"])
        .with(PlacerSystem, "placer", &["input"])
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

        world.exec(|mut dirty_entities: Write<DirtyEntities>| {
            dirty_entities.dirty.clear();
        });

        if world.read_resource::<ShouldClose>().0 {
            break 'gameloop;
        }
    }
}
