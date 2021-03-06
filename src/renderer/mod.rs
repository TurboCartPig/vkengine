pub mod camera;
pub mod geometry;
pub mod lights;

mod debug;
mod queues;
mod shaders;

use crate::{
    components::GlobalTransform,
    renderer::{
        camera::{ActiveCamera, Camera},
        debug::Debug,
        geometry::{MeshBuilder, MeshComponent, Vertex},
        lights::{DirectionalLightRes, PointLightComponent},
        queues::{QueueFamilyIds, QueueFamilyTypes},
        shaders::{Lights, PointLight, PushConstants, ShaderSet, VertexInput},
    },
    resources::DirtyEntities,
};
use log::{error, info, log_enabled, warn, Level};
use nalgebra::Vector3;
use sdl2::video::{Window as SdlWindow, WindowContext};
use shrev::{EventChannel, ReaderId};
use specs::{join::JoinIter, prelude::*};
use std::{
    cmp::{max, min},
    mem,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};
use vulkano::{
    app_info_from_cargo_toml,
    buffer::{cpu_pool::CpuBufferPool, BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::{
        descriptor_set::{FixedSizeDescriptorSetsPool, PersistentDescriptorSet},
        DescriptorSet,
    },
    device::{Device, DeviceExtensions, Features, Queue},
    format::Format,
    framebuffer::{Framebuffer, RenderPassAbstract, Subpass},
    image::{attachment::AttachmentImage, ImageUsage, SwapchainImage},
    instance::{self, Instance, InstanceExtensions, PhysicalDevice, PhysicalDeviceType},
    pipeline::{viewport::Viewport, GraphicsPipeline, GraphicsPipelineAbstract},
    single_pass_renderpass,
    swapchain::{
        self, AcquireError, CompositeAlpha, PresentMode, Swapchain, SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture, SharingMode},
    VulkanObject,
};

pub type Window = SendSyncContext;
pub type Surface = Arc<swapchain::Surface<Window>>;

pub struct SendSyncContext {
    pub _context: Rc<WindowContext>,
}

unsafe impl Send for SendSyncContext {}
unsafe impl Sync for SendSyncContext {}

trait VulkanoWindow {
    fn vulkano_surface(&self, instance: Arc<Instance>) -> Surface;
}

impl VulkanoWindow for SdlWindow {
    fn vulkano_surface(&self, instance: Arc<Instance>) -> Surface {
        let raw = unsafe {
            let surface = self
                .vulkan_create_surface(instance.internal_object())
                .unwrap();

            swapchain::Surface::from_raw_surface(
                instance,
                surface,
                SendSyncContext {
                    _context: self.context().clone(),
                },
            )
        };
        Arc::new(raw)
    }
}

#[derive(Debug)]
pub enum RenderEvent {
    WindowResized,
    StopRendering,
    StartRendering,
}

/// Resource for sharing the event channel for render events
#[derive(Default)]
pub struct RenderEvents(EventChannel<RenderEvent>);

impl Deref for RenderEvents {
    type Target = EventChannel<RenderEvent>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RenderEvents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// The main renderer
pub struct Renderer {
    pub device: Arc<Device>,
    queues: queues::Queues,
    surface: Surface,
    swapchain: Arc<Swapchain<Window>>,
    images: Vec<Arc<SwapchainImage<Window>>>,
    framebuffers: Option<
        Vec<
            Arc<
                Framebuffer<
                    Arc<dyn RenderPassAbstract + Sync + Send>,
                    (((), Arc<SwapchainImage<Window>>), Arc<AttachmentImage>),
                >,
            >,
        >,
    >,

    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    graphics_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    dynamic_state: DynamicState,

    depth_buffer: Arc<AttachmentImage>,
    vertex_input_pool: CpuBufferPool<VertexInput>,
    lights_buffer: Arc<CpuAccessibleBuffer<Lights>>,
    point_lights_buffer: Arc<CpuAccessibleBuffer<[PointLight]>>,
    descriptor_set_pool: FixedSizeDescriptorSetsPool<Arc<GraphicsPipelineAbstract + Send + Sync>>,
    shared_descriptor_set: Arc<DescriptorSet + Send + Sync>,

    previous_frame_end: Box<GpuFuture + Send + Sync>,
    event_reader: Option<ReaderId<RenderEvent>>,
    point_lights_reader_id: Option<ReaderId<ComponentEvent>>,
    should_render: bool,
    _debug: Debug,
}

impl Renderer {
    pub fn new(window: &SdlWindow) -> Self {
        let instance = new_instance();

        // We register the debug callback early in case something happens during init
        let _debug = Debug::from_instance(&instance);

        let surface = window.vulkano_surface(instance.clone()).clone();

        let (device, queues) = new_device_and_queues(instance.clone(), surface.clone());

        let (swapchain, images) =
            new_swapchain_and_images(device.clone(), surface.clone(), queues.present.clone());

        let framebuffers = None;

        let depth_buffer =
            AttachmentImage::transient(device.clone(), swapchain.dimensions(), Format::D16Unorm)
                .unwrap();
        let dynamic_state = DynamicState {
            line_width: None,
            viewports: Some(vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [
                    swapchain.dimensions()[0] as f32,
                    swapchain.dimensions()[1] as f32,
                ],
                depth_range: 0.0..1.0,
            }]),
            scissors: None,
        };

        let shaders = ShaderSet::new(device.clone());

        let render_pass = build_render_pass(device.clone(), swapchain.format());

        let graphics_pipeline =
            build_graphics_pipeline(device.clone(), render_pass.clone(), &shaders);

        let vertex_input_pool = CpuBufferPool::<VertexInput>::new(
            device.clone(),
            BufferUsage::uniform_buffer_transfer_destination(),
        );

        let dir_light = DirectionalLightRes::default().to_directional_light();

        let lights = Lights { dir_light };

        let lights_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage::uniform_buffer_transfer_destination(),
            lights,
        )
        .unwrap();

        let point_lights_buffer = {
            let usage = BufferUsage {
                storage_buffer: true,
                ..BufferUsage::none()
            };

            let point_lights = [PointLightComponent::from_color(Vector3::new(0.0, 0.0, 0.0))
                .to_point_light(Vector3::new(0.0, 0.0, 0.0))];

            CpuAccessibleBuffer::from_iter(device.clone(), usage, point_lights.iter().cloned())
                .unwrap()
        };

        let descriptor_set_pool = FixedSizeDescriptorSetsPool::new(graphics_pipeline.clone(), 0);

        let shared_descriptor_set = Arc::new(
            PersistentDescriptorSet::start(graphics_pipeline.clone(), 1)
                .add_buffer(lights_buffer.clone())
                .unwrap()
                .add_buffer(point_lights_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        let previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;

        let should_render = true;

        Self {
            device,
            queues,
            surface,
            swapchain,
            images,
            framebuffers,
            render_pass,
            graphics_pipeline,
            dynamic_state,

            depth_buffer,
            vertex_input_pool,
            lights_buffer,
            point_lights_buffer,
            descriptor_set_pool,
            shared_descriptor_set,

            previous_frame_end,
            event_reader: None,
            point_lights_reader_id: None,
            should_render,
            _debug,
        }
    }

    /// Recreates the swapchain from the old one, in case it is invalid
    pub fn recreate_swapchain(&mut self) -> Result<(), SwapchainCreationError> {
        let dimensions = {
            let caps = self
                .surface
                .capabilities(self.device.physical_device())
                .unwrap();

            let current_extent = caps.current_extent.unwrap_or(caps.min_image_extent);

            if current_extent < caps.min_image_extent {
                caps.min_image_extent
            } else if current_extent > caps.max_image_extent {
                caps.max_image_extent
            } else {
                current_extent
            }
        };

        let (new_swapchain, new_images) = self.swapchain.recreate_with_dimension(dimensions)?;

        self.depth_buffer =
            AttachmentImage::transient(self.device.clone(), dimensions, Format::D16Unorm).unwrap();

        // Converts from [i32; 2] to [f32; 2]
        let dimensions = [dimensions[0] as f32, dimensions[1] as f32];

        self.dynamic_state.viewports = Some(vec![Viewport {
            origin: [0.0, 0.0],
            dimensions: dimensions,
            depth_range: 0.0..1.0,
        }]);

        mem::replace(&mut self.swapchain, new_swapchain);
        mem::replace(&mut self.images, new_images);

        self.recreate_framebuffers();

        warn!("Swapchain recreated");

        Ok(())
    }

    /// Recreates the framebuffers backing the swapchain images inplace
    pub fn recreate_framebuffers(&mut self) {
        let new_framebuffers = Some(
            self.images
                .iter()
                .map(|image| {
                    Arc::new(
                        Framebuffer::start(self.render_pass.clone())
                            .add(image.clone())
                            .unwrap()
                            .add(self.depth_buffer.clone())
                            .unwrap()
                            .build()
                            .unwrap(),
                    )
                })
                .collect::<Vec<_>>(),
        );

        mem::replace(&mut self.framebuffers, new_framebuffers);

        warn!("Framebuffers recreated");
    }

    /// Creates a new buffer for point lights and a new descriptor set that includes it, and
    /// replace the old ones on the renderer
    fn upload_point_lights(
        &mut self,
        iter: JoinIter<(
            &ReadStorage<'_, PointLightComponent>,
            &ReadStorage<'_, GlobalTransform>,
        )>,
    ) {
        let usage = BufferUsage {
            storage_buffer: true,
            ..BufferUsage::none()
        };

        let lights = iter
            .map(|(light, global)| light.to_point_light(global.translation().clone()))
            .collect::<Vec<PointLight>>();

        let buffer =
            CpuAccessibleBuffer::from_iter(self.device.clone(), usage, lights.into_iter()).unwrap();

        let descriptor_set = Arc::new(
            PersistentDescriptorSet::start(self.graphics_pipeline.clone(), 1)
                .add_buffer(self.lights_buffer.clone())
                .unwrap()
                .add_buffer(buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        self.point_lights_buffer = buffer;
        self.shared_descriptor_set = descriptor_set;
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = (
        Entities<'a>,
        Read<'a, RenderEvents>,
        Read<'a, DirtyEntities>,
        Write<'a, DirectionalLightRes>,
        ReadStorage<'a, PointLightComponent>,
        ReadStorage<'a, GlobalTransform>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, MeshComponent>,
        WriteStorage<'a, MeshBuilder>,
        WriteStorage<'a, Camera>,
    );

    /// The main draw/render function
    fn run(
        &mut self,
        (
            entities,
            render_events,
            dirty_entities,
            mut directional_light,
            point_lights,
            globals,
            active_cameras,
            mut meshes,
            mut mesh_builders,
            mut cameras,
        ): Self::SystemData,
    ) {
        // Cleanup
        self.previous_frame_end.cleanup_finished();

        // FIXME This seems like a hack and not the proper way to do this
        // Swap the GpuFuture out of the Renderer
        let mut frame_future = Box::new(sync::now(self.device.clone())) as Box<_>;
        mem::swap(&mut frame_future, &mut self.previous_frame_end);

        // Handle render events
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        render_events
            .read(self.event_reader.as_mut().unwrap())
            .for_each(|event| {
                warn!("Render event: {:?}", event);
                match event {
                    RenderEvent::WindowResized => {
                        self.recreate_swapchain().unwrap();
                    }
                    RenderEvent::StopRendering => {
                        self.should_render = false;
                    }
                    RenderEvent::StartRendering => {
                        self.should_render = true;
                    }
                    // _ => (),
                }
            });

        if !self.should_render {
            return;
        }

        // TODO Find out if this is only needed for init or if we need to check for this each frame
        if self.framebuffers.is_none() {
            self.recreate_framebuffers();
        }

        // Acquire image to draw final frame to
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        let (image_number, acquired_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(ret) => ret,
                // Can happen if the user has resized the window
                Err(AcquireError::OutOfDate) => {
                    error!("Swapchain out of date");
                    self.recreate_swapchain().unwrap();
                    return;
                }
                Err(err) => panic!("Error occurred while acquiring next image: {:?}", err),
            };

        // Camera
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        let (camera, camera_t) = {
            // FIXME What if there is more than one camera?
            let (camera, camera_t, _) = (&mut cameras, &globals, &active_cameras)
                .join()
                .next()
                .unwrap();

            let dimensions = self.swapchain.dimensions();
            camera.update_aspect({ dimensions[0] as f32 / dimensions[1] as f32 });

            (camera, camera_t)
        };

        // Mesh building
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        {
            // Build mesh components from mesh builders
            (&entities, &globals, &mesh_builders.mask().clone())
                .join()
                .for_each(|(entity, global, _)| {
                    let builder = mesh_builders.remove(entity).unwrap();

                    let vertex = VertexInput {
                        // model: global.to_view_matrix().into(),
                        model: global.to_matrix().into(),
                    };

                    let mesh = builder.build(
                        self.device.clone(),
                        &self.vertex_input_pool,
                        vertex,
                        &mut self.descriptor_set_pool,
                    );

                    meshes.insert(entity, mesh).unwrap();
                });
        }

        // Update buffers
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        let buffer_update_command_buffer = {
            let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
                self.device.clone(),
                self.queues.present.family(),
            )
            .unwrap();

            // Uniforms
            // -----------------------------------------------------------------------------------------------------------------------------------------------------------

            builder = (&meshes, &globals, &dirty_entities.dirty).join().fold(
                builder,
                |builder, (mesh, global, _)| {
                    let vertex = VertexInput {
                        // model: global.to_view_matrix().into(),
                        model: global.to_matrix().into(),
                    };

                    builder
                        .update_buffer(mesh.vertex_uniforms.clone(), vertex)
                        .unwrap()
                },
            );

            // Directional light
            // -----------------------------------------------------------------------------------------------------------------------------------------------------------

            // Update the lights buffer if the directional light has changed
            if directional_light.dirty {
                directional_light.dirty = false;

                let lights = Lights {
                    dir_light: directional_light.to_directional_light(),
                };

                builder = builder
                    .update_buffer(self.lights_buffer.clone(), lights)
                    .unwrap();
            }

            builder.build().unwrap()
        };

        // Flush and submit command buffers
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        let frame_future = frame_future
            .then_execute(self.queues.present.clone(), buffer_update_command_buffer)
            .unwrap()
            .then_signal_semaphore_and_flush()
            .unwrap();

        // Point lights
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        {
            let mut should_update = false;

            // TODO Only upload changes.
            // Insert and Remove events should be handled as is, but modified events do not require
            // a new buffer, we can simply write the changes to the buffer
            point_lights
                .channel()
                .read(self.point_lights_reader_id.as_mut().unwrap())
                .for_each(|event| match *event {
                    _ => {
                        should_update = true;
                    }
                });

            if should_update {
                self.upload_point_lights((&point_lights, &globals).join());
            }
        }

        // Push constants
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        let pc = PushConstants {
            view: camera_t.to_view_matrix().into(),
            proj: camera.projection(),
        };

        // Drawing
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        // Build a primary command buffer builder
        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
            self.device.clone(),
            self.queues.present.family(),
        )
        .unwrap()
        .begin_render_pass(
            self.framebuffers.as_ref().unwrap()[image_number].clone(),
            true, // This makes it so that we can execute secondary command buffers
            vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()],
        )
        .unwrap();

        // Build secondary command buffers and execute them in the primary command buffer.
        // Then build the primary command buffer
        let secondary_command_buffers = (&meshes)
            .par_join()
            .map(|mesh| {
                let descriptor_sets = vec![
                    mesh.descriptor_set.clone(),
                    self.shared_descriptor_set.clone(),
                ];

                let secondary_command_buffer =
                    AutoCommandBufferBuilder::secondary_graphics_one_time_submit(
                        self.device.clone(),
                        self.queues.present.family(),
                        self.graphics_pipeline.clone().subpass(),
                    )
                    .unwrap()
                    // .draw(
                    //     self.graphics_pipeline.clone(),
                    //     &self.dynamic_state,
                    //     vec![mesh.vertex_buffer.clone()],
                    //     descriptor_sets,
                    //     pc,
                    // )
                    .draw_indexed(
                        self.graphics_pipeline.clone(),
                        &self.dynamic_state,
                        vec![mesh.vertex_buffer.clone()],
                        mesh.index_buffer.clone(),
                        descriptor_sets,
                        pc,
                    )
                    .unwrap()
                    .build()
                    .unwrap();

                secondary_command_buffer
            })
            .collect::<Vec<_>>();

        let command_buffer = secondary_command_buffers
            .into_iter()
            .fold(
                command_buffer,
                |command_buffer, secondary_command_buffer| {
                    let command_buffer = unsafe {
                        command_buffer
                            .execute_commands(secondary_command_buffer)
                            .unwrap()
                    };

                    command_buffer
                },
            )
            .end_render_pass()
            .unwrap()
            .build()
            .unwrap();

        // Presenting
        // -----------------------------------------------------------------------------------------------------------------------------------------------------------

        let frame_future = {
            let present_future = frame_future
                .join(acquired_future)
                .then_execute(self.queues.present.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(
                    self.queues.present.clone(),
                    self.swapchain.clone(),
                    image_number,
                )
                .then_signal_fence_and_flush();

            match present_future {
                Ok(future) => Box::new(future) as Box<GpuFuture + Send + Sync>,
                Err(FlushError::OutOfDate) => {
                    error!("Swapchain out of date");
                    self.recreate_swapchain().unwrap();
                    Box::new(sync::now(self.device.clone())) as Box<_>
                }
                Err(err) => {
                    error!("{:?}", err);
                    Box::new(sync::now(self.device.clone())) as Box<_>
                }
            }
        };

        // Store the GpuFuture in Renderer again
        mem::replace(&mut self.previous_frame_end, frame_future);
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);

        // Register readers
        {
            let mut render_events = res.fetch_mut::<RenderEvents>();
            self.event_reader = Some(render_events.register_reader());

            let mut point_lights = WriteStorage::<PointLightComponent>::fetch(res);
            self.point_lights_reader_id = Some(point_lights.register_reader());
        }

        // Upload the point lights that exists before setup() is called. If we don't do this, the
        // point lights buffer is not in sync with the ecs world
        {
            let point_lights = ReadStorage::<PointLightComponent>::fetch(res);
            let globals = ReadStorage::<GlobalTransform>::fetch(res);

            self.upload_point_lights((&point_lights, &globals).join());
        }
    }
}

/// Creates a vulkan instance based on desired extensions and layers
///
/// # Panics
///
/// - Panics if desired layer is not available
/// - Panics if a core extension failes to load
/// - Panics if instance can not be created
fn new_instance() -> Arc<instance::Instance> {
    let info = app_info_from_cargo_toml!();

    let extensions = {
        let desired = InstanceExtensions {
            // Generic
            khr_surface: true,
            khr_display: true,

            // Linux
            khr_xlib_surface: true,
            khr_xcb_surface: true,
            khr_wayland_surface: true,

            // Android
            khr_android_surface: true,

            // Windows
            khr_win32_surface: true,

            // Apple
            mvk_ios_surface: true,
            mvk_macos_surface: true,

            // Debugging
            ext_debug_report: true,

            ..InstanceExtensions::none()
        };

        let supported = InstanceExtensions::supported_by_core()
            .expect("Failed to load supported instance extensions");

        supported.intersection(&desired)
    };

    let layers = {
        let available = instance::layers_list().unwrap().collect::<Vec<_>>();

        let desired = vec![
            //"VK_LAYER_LUNARG_api_dump",
            //"VK_LAYER_LUNARG_core_validation",
            //"VK_LAYER_LUNARG_device_simulation",
            //"VK_LAYER_LUNARG_monitor",
            //"VK_LAYER_LUNARG_object_tracker",
            //"VK_LAYER_LUNARG_parameter_validation",
            //"VK_LAYER_LUNARG_screenshot",
            "VK_LAYER_LUNARG_standard_validation",
            //"VK_LAYER_LUNARG_vktrace",
            //"VK_LAYER_VALVE_steam_overlay",
            //"VK_LAYER_RENDERDOC_Capture",
        ];

        if log_enabled!(Level::Info) {
            info!("Available instance layers:\n");
            for layer in available.iter() {
                info!(
                    "Layer Name: {}\nLayer Version: {}\nLayer Description: {}\n",
                    layer.name(),
                    layer.implementation_version(),
                    layer.description()
                );
            }
        }

        // Panics if a desired layer is not available
        for dlayer in desired.iter() {
            available
                .iter()
                .find(|alayer| &alayer.name() == dlayer)
                .expect("Failed to find desired validation layer");
        }

        desired
    };

    instance::Instance::new(Some(&info), &extensions, layers)
        .expect("Failed to create vulkan instance")
}

/// Creates a logical device and its queues
///
/// # Panics
///
/// - Panics if required features are not supported
/// - Panics if required device extensions are not supported
fn new_device_and_queues(
    instance: Arc<instance::Instance>,
    surface: Surface,
) -> (Arc<Device>, queues::Queues) {
    let (physical, queue_family_ids) = {
        info!("Listing enumerated devices...\n");

        // TODO Tune scores
        let mut devices = PhysicalDevice::enumerate(&instance)
            .map(|device| {
                let mut score = 0u32;

                // Score for device type
                match device.ty() {
                    PhysicalDeviceType::DiscreteGpu => score += 10_000,
                    PhysicalDeviceType::IntegratedGpu => score += 5_000,
                    _ => (),
                }

                // Score for device api version
                // The points given by this should not impact the score much as most implementations are kept up to date
                let ver = device.api_version();
                score += (ver.major * 1_000) as u32;
                score += (ver.minor * 100) as u32;
                score += (ver.patch * 2) as u32;

                // We assume that the queue families are all unique
                // For Nvidia gpus we can expect 16 general queues
                // For Intel gpus we can expect 1 general queue
                // For AMD gpus we can expect 1 graphics, ~8 compute and ~2 transfer queues
                // But the only expectation we can assume is Nvidias
                let queue_family_ids = QueueFamilyIds::from_queue_families_iter(
                    device.queue_families(),
                    surface.clone(),
                );

                (device, score, queue_family_ids)
            })
            .inspect(|(device, score, _)| {
                info!(
                    "\
                     Device name: {}\n\
                     Device type: {:?}\n\
                     Device api version: {:?}\n\
                     Device score: {}\n",
                    device.name(),
                    device.ty(),
                    device.api_version(),
                    score
                );
            })
            .collect::<Vec<_>>();

        // Sort them by score (Highest score last)
        devices.sort_by(|(_, a, _), (_, b, _)| a.cmp(&b));

        let (physical, score, queue_family_ids) = devices.pop().unwrap();
        assert_ne!(score, 0u32); // If score = 0, it means we failed to find a suitable gpu

        (physical, queue_family_ids)
    };

    info!("Physical device chosen: {:?}\n", physical.name());

    let (queues, queue_types) = {
        let queues_count = physical.queue_families().len();
        let mut queues = Vec::with_capacity(queues_count);
        let mut queue_types = Vec::with_capacity(queues_count);

        // Adds 4 general queues or 1 general, 1 graphics, 1 compute and 1 present queue
        // All of this is more to experiment with vulkan and implamentations than anything else
        // we could probably just stick to one queue, but this is more fun :)
        if let Some(id) = queue_family_ids.general {
            let qf = physical.queue_family_by_id(id).unwrap();

            for _ in 0..min(4, qf.queues_count()) {
                queues.push((qf, 1.0f32));
                queue_types.push(QueueFamilyTypes::General);
            }
        } else if queues.is_empty() {
            if let Some(id) = queue_family_ids.graphics {
                let qf = physical.queue_family_by_id(id).unwrap();

                queues.push((qf, 1.0f32));
                queue_types.push(QueueFamilyTypes::Graphics);
            }
            if let Some(id) = queue_family_ids.compute {
                let qf = physical.queue_family_by_id(id).unwrap();

                queues.push((qf, 1.0f32));
                queue_types.push(QueueFamilyTypes::Compute);
            }
        }
        if let Some(id) = queue_family_ids.present {
            let qf = physical.queue_family_by_id(id).unwrap();

            queues.push((qf, 1.0f32));
            queue_types.push(QueueFamilyTypes::Present);
        }

        (queues, queue_types)
    };

    info!("Queues to be created: {:?}", queues.len());
    info!("Queue types to be created: {:?}", queue_types);

    let features = {
        // TODO: Check for minimum required features
        let required_features = Features {
            fill_mode_non_solid: true,
            ..Features::none()
        };

        let optimal_features = Features::all();

        // Panic if desired features are not supported
        assert!(physical
            .supported_features()
            .superset_of(&required_features));
        assert!(optimal_features.superset_of(&required_features));

        optimal_features.intersection(physical.supported_features())
    };

    let extensions = {
        let required_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        DeviceExtensions::supported_by_device(physical).intersection(&required_extensions)
    };

    let (device, queues_iter) = Device::new(physical, &features, &extensions, queues)
        .expect("Failed to create logical device");

    // FIXME What if there are more then one general queue
    let queues = {
        let queues = queues_iter.collect::<Vec<_>>();

        // All vulkan implementations have to have one general queue
        // Therefor it is ok to panic if we cant get one
        let general = queues[0].clone();

        let compute = if queue_types.get(1) == Some(&QueueFamilyTypes::Compute)
            || queue_types.get(1) == Some(&QueueFamilyTypes::General)
        {
            queues[1].clone()
        } else {
            general.clone()
        };

        let graphics = if queue_types.get(2) == Some(&QueueFamilyTypes::Graphics)
            || queue_types.get(2) == Some(&QueueFamilyTypes::General)
        {
            queues[2].clone()
        } else {
            general.clone()
        };

        let present = if queue_types.get(3) == Some(&QueueFamilyTypes::Present)
            || queue_types.get(3) == Some(&QueueFamilyTypes::General)
        {
            queues[3].clone()
        } else {
            general.clone()
        };

        queues::Queues {
            general,
            compute,
            graphics,
            present,
        }
    };

    (device, queues)
}

/// Cretes new swapchain and its images
///
/// # Panics
///
/// - Panics if required capabilities are not present
/// - Panics if swapchain creation failes
fn new_swapchain_and_images(
    device: Arc<Device>,
    surface: Surface,
    queue: Arc<Queue>,
) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
    let capabilities = surface
        .capabilities(device.physical_device())
        .expect("Failed to get surface capabilities");

    info!("Surface capabilities: {:?}\n", capabilities);

    let buffer_count = max(
        capabilities.min_image_count,
        capabilities
            .max_image_count
            .unwrap_or(capabilities.min_image_count),
    );

    // First available format
    let format = capabilities.supported_formats[0].0;
    // info!("Supported formats: {:?}", capabilities.supported_formats);

    // Current extent seems to be the screen res normaly
    // FIXME The dimensions dont match the inner window size
    let dimensions = capabilities.current_extent.unwrap_or([1600, 900]);

    // We will only use this image for color
    let image_usage = ImageUsage {
        color_attachment: true,
        ..ImageUsage::none()
    };

    // Only our present queue needs access to this image
    let sharing_mode = SharingMode::Exclusive(queue.family().id());

    // We dont need support for flipping the window or anything similar
    let transform = capabilities.current_transform;

    // We prefer a non-transparent window
    let alpha_composite = if capabilities
        .supported_composite_alpha
        .supports(CompositeAlpha::Opaque)
    {
        CompositeAlpha::Opaque
    } else if capabilities
        .supported_composite_alpha
        .supports(CompositeAlpha::Inherit)
    {
        CompositeAlpha::Inherit
    } else {
        capabilities
            .supported_composite_alpha
            .iter()
            .next()
            .unwrap()
    };

    // We prefer Mailbox, then Fifo
    let present_mode = if capabilities.present_modes.supports(PresentMode::Mailbox) {
        PresentMode::Mailbox
    } else if capabilities.present_modes.supports(PresentMode::Fifo) {
        PresentMode::Fifo
    } else {
        capabilities.present_modes.iter().next().unwrap()
    };

    Swapchain::new(
        device.clone(),
        surface.clone(),
        buffer_count,
        format,
        dimensions,
        1,
        image_usage,
        sharing_mode,
        transform,
        alpha_composite,
        present_mode,
        true,
        None,
    )
    .expect("Failed to create swapchain")
}

fn build_render_pass(device: Arc<Device>, format: Format) -> Arc<RenderPassAbstract + Send + Sync> {
    Arc::new(
        single_pass_renderpass!(device.clone(),
            attachments: {
                // `color` is a custom name
                color: {
                    load: Clear,
                    store: Store,
                    format: format,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .unwrap(),
    )
}

fn build_graphics_pipeline(
    device: Arc<Device>,
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    shaders: &ShaderSet,
) -> Arc<GraphicsPipelineAbstract + Send + Sync> {
    let sc = shaders::FragSC { gamma: 2.2 };

    Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(shaders.vertex.main_entry_point(), ())
            .triangle_list()
            //.polygon_mode_line()
            .viewports_dynamic_scissors_irrelevant(1)
            // .cull_mode_back()
            .fragment_shader(shaders.fragment.main_entry_point(), sc)
            .depth_stencil_simple_depth()
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device.clone())
            .unwrap(),
    )
}
