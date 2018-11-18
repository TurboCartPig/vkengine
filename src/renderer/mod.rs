pub mod camera;
pub mod geometry;
mod queues;
mod shaders;

use self::{
    camera::{ActiveCamera, Camera},
    geometry::MeshComponent,
    queues::{QueueFamilyIds, QueueFamilyTypes},
    shaders::ShaderSet,
};
use components::Transform;
use resources::DeltaTime;
use specs::prelude::*;
use std::{
    cmp::{max, min},
    mem,
    sync::{Arc, RwLock},
};
use vulkano::{
    app_info_from_cargo_toml,
    buffer::{cpu_pool::CpuBufferPool, BufferUsage},
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    descriptor::descriptor_set::FixedSizeDescriptorSetsPool,
    device::{Device, DeviceExtensions, Features},
    format::Format,
    framebuffer::{Framebuffer, RenderPassAbstract, Subpass},
    image::{attachment::AttachmentImage, SwapchainImage},
    impl_vertex,
    instance::{
        self,
        debug::{DebugCallback, MessageTypes},
        InstanceExtensions, PhysicalDevice, PhysicalDeviceType,
    },
    pipeline::{viewport::Viewport, GraphicsPipeline, GraphicsPipelineAbstract},
    single_pass_renderpass,
    swapchain::{self, AcquireError, Swapchain, SwapchainCreationError},
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{EventsLoop, Window, WindowBuilder};

pub type Surface = Arc<swapchain::Surface<Window>>;

#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl_vertex!(Vertex, position, normal);

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
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    graphics_pipeline: Arc<GraphicsPipelineAbstract + Send + Sync>,
    dynamic_state: DynamicState,
    depth_buffer: Arc<AttachmentImage>,
    uniform_buffer_pool: CpuBufferPool<shaders::VertexInput>,
    descriptor_set_pool: FixedSizeDescriptorSetsPool<Arc<GraphicsPipelineAbstract + Send + Sync>>,
    previous_frame_end: Box<GpuFuture + Send + Sync>,
    //_callback: Option<DebugCallback>,
}

impl Renderer {
    pub fn new(events_loop: &EventsLoop) -> Self {
        let instance = new_instance();

        // We regiser the debug callback early in case something happens during init
        //let _callback = register_debug_callback(instance.clone());

        //let monitor = events_loop.get_primary_monitor();

        let surface = WindowBuilder::new()
            .with_title("VK Engine")
            //.with_maximized(true)
            //.with_fullscreen(Some(monitor))
            //.with_dimensions()
            .build_vk_surface(events_loop, instance.clone())
            .unwrap();

        let (device, queues) = new_device_and_queues(instance.clone(), surface.clone());

        let (swapchain, images) =
            new_swapchain_and_images(device.clone(), surface.clone(), &queues);

        let framebuffers = None;

        let shaders = ShaderSet::new(device.clone());

        let render_pass = build_render_pass(device.clone(), swapchain.format());

        let graphics_pipeline =
            build_graphics_pipeline(device.clone(), render_pass.clone(), &shaders);

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

        let uniform_buffer_pool =
            CpuBufferPool::<shaders::VertexInput>::new(device.clone(), BufferUsage::all());

        let descriptor_set_pool = FixedSizeDescriptorSetsPool::new(graphics_pipeline.clone(), 0);

        let depth_buffer =
            AttachmentImage::transient(device.clone(), swapchain.dimensions(), Format::D16Unorm)
                .unwrap();

        let previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;

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
            uniform_buffer_pool,
            descriptor_set_pool,
            previous_frame_end,
            //_callback,
        }
    }

    pub fn surface(&self) -> Surface {
        self.surface.clone()
    }

    fn recreate_swapchain(&mut self) -> Result<(), SwapchainCreationError> {
        let dimensions = self
            .surface
            .capabilities(self.device.physical_device())
            .unwrap()
            .current_extent
            .unwrap_or([1600, 900]);

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

        println!("INFO: Swapchain recreated");
        Ok(())
    }

    fn recreate_framebuffers(&mut self) {
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
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = (
        ReadStorage<'a, MeshComponent>,
        ReadStorage<'a, Transform>,
        ReadStorage<'a, ActiveCamera>,
        WriteStorage<'a, Camera>,
        Read<'a, DeltaTime>,
    );

    fn run(&mut self, (mesh, transform, active_camera, mut camera, _delta_time): Self::SystemData) {
        self.previous_frame_end.cleanup_finished();

        // TODO Find out if this is only needed for init or if we need to check for this each frame
        if self.framebuffers.is_none() {
            self.recreate_framebuffers();
        }

        // Acquire image to draw final frame to
        let (image_number, acquired_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(ret) => ret,
                // Can happen if the user has resized the window
                Err(AcquireError::OutOfDate) => {
                    println!("ERROR: Swapchain out of date");
                    self.recreate_swapchain().unwrap();
                    return;
                }
                Err(err) => panic!("Error occurred while acquiring next image: {:?}", err),
            };

        // Camera
        let (_, camera, camera_t) = (&active_camera, &mut camera, &transform)
            .join()
            .next()
            .unwrap();
        let dimensions = self.swapchain.dimensions();
        camera.update_aspect({ dimensions[0] as f32 / dimensions[1] as f32 });
        let view = camera_t.to_view_matrix();

        let secondary_command_buffers = RwLock::new(Vec::with_capacity(2usize));

        for (mesh, transform) in (&mesh, &transform).join() {
            let uniform_buffer_subbuffer = {
                let model = transform.to_matrix();

                let uniform_data = shaders::VertexInput {
                    view: view.into(),
                    proj: camera.projection(),
                    model: model.into(),
                };

                self.uniform_buffer_pool.next(uniform_data).unwrap()
            };

            // FIXME Make persistant descriptor sets and put them on the MeshComponent
            let descriptor_set = self
                .descriptor_set_pool
                .next()
                .add_buffer(uniform_buffer_subbuffer)
                .unwrap()
                .build()
                .unwrap();

            let secondary_command_buffer =
                AutoCommandBufferBuilder::secondary_graphics_one_time_submit(
                    self.device.clone(),
                    self.queues.graphics.family(),
                    Subpass::from(self.render_pass.clone(), 0).unwrap(),
                )
                .unwrap()
                .draw(
                    self.graphics_pipeline.clone(),
                    &self.dynamic_state,
                    vec![mesh.vertex_buffer.clone()],
                    descriptor_set,
                    (),
                )
                .unwrap()
                .build()
                .unwrap();

            {
                let mut scbg = secondary_command_buffers.write().unwrap();
                scbg.push(secondary_command_buffer);
            }
        }

        // FIXME This seems like a hack and not the proper way to do this
        // Swap the GpuFuture out of the Renderer
        let mut previous_frame_end = Box::new(sync::now(self.device.clone())) as Box<_>;
        mem::swap(&mut previous_frame_end, &mut self.previous_frame_end);

        let secondary_command_buffers = secondary_command_buffers.into_inner().unwrap();

        if secondary_command_buffers.len() == 0 {
            return;
        }

        let command_buffer = {
            let mut command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
                self.device.clone(),
                self.queues.graphics.family(),
            )
            .unwrap()
            .begin_render_pass(
                self.framebuffers.as_ref().unwrap()[image_number].clone(),
                true, // This makes it so that I can execute secondary command buffers
                vec![[0.0, 0.0, 0.0, 1.0].into(), 1f32.into()],
            )
            .unwrap();

            unsafe {
                // Execute all the secondary command buffers
                for scb in secondary_command_buffers {
                    command_buffer = command_buffer.execute_commands(scb).unwrap();
                }
            }

            command_buffer.end_render_pass().unwrap().build().unwrap()
        };

        let present_future = previous_frame_end
            .join(acquired_future)
            .then_execute(self.queues.present.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queues.present.clone(),
                self.swapchain.clone(),
                image_number,
            )
            .then_signal_fence_and_flush();

        previous_frame_end = match present_future {
            Ok(future) => Box::new(future) as Box<_>,
            Err(FlushError::OutOfDate) => {
                println!("ERROR: Swapchain out of date");
                self.recreate_swapchain().unwrap();
                Box::new(sync::now(self.device.clone())) as Box<_>
            }
            Err(err) => {
                println!("{:?}", err);
                Box::new(sync::now(self.device.clone())) as Box<_>
            }
        };

        // Store the GpuFuture in Renderer again
        mem::replace(&mut self.previous_frame_end, previous_frame_end);
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
    }
}

#[allow(dead_code)]
fn register_debug_callback(instance: Arc<instance::Instance>) -> Option<DebugCallback> {
    let message_types = MessageTypes {
        error: true,
        warning: true,
        performance_warning: true,
        information: false,
        debug: true,
    };

    DebugCallback::new(&instance, message_types, |msg| {
        println!(
            "Debug callback from {}: {}",
            msg.layer_prefix, msg.description
        );
    })
    .ok()
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
            "VK_LAYER_VALVE_steam_overlay",
        ];

        // Panics if a desired layer is not available
        for dlayer in desired.clone() {
            let mut available = instance::layers_list().unwrap();

            println!("Available instance layers");

            available
                .inspect(|alayer| println!("\t{:?}", alayer.name()))
                .find(|alayer| alayer.name() == dlayer)
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
        println!("Listing enumerated devices...\n");

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
                println!(
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

    println!("Physical device chosen: {:?}\n", physical.name());

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
        } else if queues.len() < 1 {
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

    println!("Queues to be created: {:?}", queues.len());
    println!("Queue types to be created: {:?}", queue_types);

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
    queues: &queues::Queues,
) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
    use vulkano::{
        image::ImageUsage,
        swapchain::{CompositeAlpha, PresentMode, Swapchain},
        sync::SharingMode,
    };

    let capabilities = surface
        .capabilities(device.physical_device())
        .expect("Failed to get surface capabilities");

    //println!("Surface capabilities: {:?}\n", capabilities);

    let buffer_count = max(
        capabilities.min_image_count,
        capabilities
            .max_image_count
            .unwrap_or(capabilities.min_image_count),
    );

    // First available format
    let format = capabilities.supported_formats[0].0;
    println!("Supported formats: {:?}", capabilities.supported_formats);

    // Current extent seems to be the screen res normaly
    // FIXME The dimensions dont match the inner window size
    let dimensions = capabilities.current_extent.unwrap_or([1600, 900]);

    // We will only use this image for color
    let image_usage = ImageUsage {
        color_attachment: true,
        ..ImageUsage::none()
    };

    // Only our present queue needs access to this image
    let sharing_mode = SharingMode::Exclusive(queues.present.family().id());

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
    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(shaders.vertex.main_entry_point(), ())
            .triangle_list()
            //.polygon_mode_line()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(shaders.fragment.main_entry_point(), ())
            .depth_stencil_simple_depth()
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    pipeline
}
