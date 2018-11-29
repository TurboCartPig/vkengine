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
        Renderer,
    },
    resources::{FocusGained, Keyboard, Mouse, ShouldClose, Time},
    systems::{FlyControlSystem, TimeSystem, TransformSystem},
};
use log::{info, warn};
use nalgebra::Vector3;
use specs::prelude::*;
use specs_hierarchy::HierarchySystem;

//TODO Mesh loading
//TODO Use glyph-brush insted of vulkano_text
//TODO Fix/Impl lighting

use crate::renderer::Surface;
use sdl2::{
    controller::GameController,
    event::{Event, WindowEvent},
    video::{Window, WindowContext, FullscreenType},
    keyboard::Keycode,
    EventPump, GameControllerSubsystem, Sdl, VideoSubsystem,
};
use std::{rc::Rc, sync::Arc, thread};
use vulkano::{instance::Instance, swapchain, VulkanObject};

pub struct SendWindowContext {
    _context: Rc<WindowContext>,
    id: thread::ThreadId,
}

unsafe impl Send for SendWindowContext {}
unsafe impl Sync for SendWindowContext {}

impl SendWindowContext {
    pub fn new(_context: Rc<WindowContext>) -> Self {
        Self {
            _context,
            id: thread::current().id(),
        }
    }
}

impl Drop for SendWindowContext {
    fn drop(&mut self) {
        if thread::current().id() != self.id {
            unreachable!("Drop called from wrong thread");
        }
    }
}

pub trait VulkanoWindow {
    fn vulkano_surface(&self, instance: Arc<Instance>) -> Surface;
}

impl VulkanoWindow for Window {
    fn vulkano_surface(&self, instance: Arc<Instance>) -> Surface {
        let raw = unsafe {
            let handle = self
                .vulkan_create_surface(instance.internal_object())
                .unwrap();
            swapchain::Surface::from_raw_surface(
                instance,
                handle,
                SendWindowContext::new(self.context()),
            )
        };
        Arc::new(raw)
    }
}

struct SDLSystem {
    context: Sdl,
    window: Window,
    video_subsystem: VideoSubsystem,
    controller_subsystem: GameControllerSubsystem,
    controllers: Vec<GameController>,
    event_pump: EventPump,
}

impl SDLSystem {
    pub fn new() -> Self {
        let context = sdl2::init().unwrap();
        let video_subsystem = context.video().unwrap();
        let controller_subsystem = context.game_controller().unwrap();
        let event_pump = context.event_pump().unwrap();

        context.mouse().set_relative_mouse_mode(true);

        let window = video_subsystem
            .window("vkengine", 1600, 900)
            .resizable()
            .position_centered()
            .input_grabbed()
            .allow_highdpi()
            .vulkan()
            .build()
            .unwrap();

        let controllers = Vec::new();
        Self {
            context,
            window,
            video_subsystem,
            controller_subsystem,
            controllers,
            event_pump,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}

impl<'a> System<'a> for SDLSystem {
    type SystemData = (
        Write<'a, ShouldClose>,
        Write<'a, FocusGained>,
        Write<'a, Keyboard>,
        Write<'a, Mouse>,
    );

    fn run(
        &mut self,
        (mut should_close, mut window_focus, mut keyboard, mut mouse): Self::SystemData,
    ) {
        // Reset
        mouse.clear_deltas();

        let mouse_util = &self.context.mouse();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => should_close.0 = true,
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::FocusGained => {
                        window_focus.0 = true;
                        mouse_util.capture(true);
                        mouse_util.show_cursor(false);
                    }
                    WindowEvent::FocusLost => {
                        window_focus.0 = false;
                        mouse_util.capture(false);
                        mouse_util.show_cursor(true);

                        mouse.clear_all();
                        keyboard.clear_all();
                    }
                    _ => (),
                },
                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    mouse.absolute = (x, y);
                    mouse.delta = (xrel, yrel);
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    mouse.set_pressed(mouse_btn, true);
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    mouse.set_pressed(mouse_btn, false);
                }
                Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                    should_close.0 = true;
                }
                Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                    self.window.set_fullscreen(FullscreenType::Desktop).unwrap();
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    keyboard.set_pressed(key, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    keyboard.set_pressed(key, false);
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    let name = self.controller_subsystem.name_for_index(which).unwrap();
                    info!("Found game controller: {}", name);

                    let controller = self.controller_subsystem.open(which).unwrap();
                    self.controllers.push(controller);
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    // Find index of controller to remove
                    let idx = self.controllers.iter().enumerate().find(|(_, c)| c.instance_id() == which).map(|(idx, _)| idx).unwrap();
                    self.controllers.remove(idx);
                }
                _ => (),
            }
        }
    }
}

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
