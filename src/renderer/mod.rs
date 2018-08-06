mod vulkano_win;
mod shaders;
mod queues;

use self::vulkano_win::VkSurfaceBuild;
use self::queues::{QueueFamilyTypes, QueueFamilyIds};
use self::shaders::ShaderSet;

use winit::{Window, WindowBuilder, EventsLoop};

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder};
use vulkano::framebuffer::{Framebuffer, Subpass, RenderPassAbstract};
use vulkano::instance;
use vulkano::instance::{PhysicalDevice, PhysicalDeviceType, DeviceExtensions, InstanceExtensions};
use vulkano::instance::debug::{DebugCallback, MessageTypes};
use vulkano::device::{Device};
use vulkano::sync;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::pipeline::viewport::Viewport;
use vulkano::swapchain;
use vulkano::swapchain::{AcquireError, SwapchainCreationError, Swapchain};
use vulkano::image::SwapchainImage;
use vulkano::format::Format;

use std::cmp::{min, max};
use std::mem;
use std::sync::Arc;

pub type Surface = Arc<swapchain::Surface<Window>>;

#[derive(Debug, Clone)]
struct Vertex {
    position: [f32; 2]
}

pub struct Renderer {
    pub device: Arc<Device>,
    queues: queues::Queues,
    surface: Surface,
    swapchain: Arc<Swapchain<Window>>,
    images: Vec<Arc<SwapchainImage<Window>>>,
    //framebuffers: Option<Vec<Arc<FramebufferAbstract + Send + Sync>>>,
    framebuffers: Option<Vec<Arc<Framebuffer<Arc<dyn RenderPassAbstract + Sync + Send>, ((), Arc<SwapchainImage<Window>>)>>>>,
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    graphics_pipeline: Arc<GraphicsPipelineAbstract + Send + Sync>,

    dynamic_state: DynamicState,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,

    _callback: Option<DebugCallback>,
}

impl Renderer {
    pub fn new(events_loop: &EventsLoop) -> Self {
        let instance = Self::new_instance();

        let surface = WindowBuilder::new()
            .with_title("VK Engine")
            .build_vk_surface(events_loop, instance.clone())
            .unwrap();

        let (device, queues) = Self::new_device_and_queues(instance.clone(), surface.clone());

        let (swapchain, images) = Self::new_swapchain_and_images(device.clone(), surface.clone(), &queues);

        let framebuffers = None;

        let shaders = Self::load_shaders(device.clone());

        let render_pass = Self::build_render_pass(device.clone(), swapchain.format());

        let graphics_pipeline = Self::build_graphics_pipeline(device.clone(), render_pass.clone(), &shaders);

        let dynamic_state = DynamicState {
            line_width: None,
            // TODO: Find a way to do this without having to dynamically allocate a Vec every frame.
            viewports: Some(vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [swapchain.dimensions()[0] as f32, swapchain.dimensions()[1] as f32],
                depth_range: 0.0 .. 1.0,
            }]),
            scissors: None,
        };

        // The buffer contains a triangle
        let vertex_buffer = {
            impl_vertex!(Vertex, position);

            CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), [
                Vertex { position: [ -0.5, -0.25] },
                Vertex { position: [ 0.0,   0.5 ] },
                Vertex { position: [ 0.25, -0.1 ] }
            ]
            .iter()
            .cloned())
            .expect("failed to create buffer")
        };

        #[cfg(feature = "with-debuging")]
        let _callback = register_debug_callback(&device.instance);
        #[cfg(not(feature = "with-debugging"))]
        let _callback = None;

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
            vertex_buffer,

            _callback,
        }
    }

    pub fn render(&mut self, mut previous_frame_end: Box<GpuFuture>) -> Box<GpuFuture> {
        previous_frame_end.cleanup_finished();

        if self.framebuffers.is_none() {
            let new_framebuffers = Some(self.images.iter().map(|image| {
                Arc::new(Framebuffer::start(self.render_pass.clone())
                         .add(image.clone()).unwrap()
                         .build().unwrap())
            }).collect::<Vec<_>>());

            mem::replace(&mut self.framebuffers, new_framebuffers);
        }

        let (image_number, acquired_future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(ret) => ret,
            // Can happen if the user has resized the window
            Err(AcquireError::OutOfDate) => {
                println!("ERROR: Swapchain out of date");
                self.recreate_swapchain();
                return previous_frame_end;
            },
            Err(err) => panic!("Error occurred while acquiring next image: {:?}", err),
        };

        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queues.graphics.family()).unwrap()
            .begin_render_pass(self.framebuffers.as_ref().unwrap()[image_number].clone(), false, vec![[0.0, 0.0, 1.0, 1.0].into()])
            .unwrap()
                .draw(self.graphics_pipeline.clone(),
                      self.dynamic_state.clone(),
                      vec!(self.vertex_buffer.clone()),
                      (),
                      ())
                .unwrap()
            .end_render_pass()
            .unwrap()
            .build()
            .unwrap();

        let present_future = previous_frame_end.join(acquired_future)
            .then_execute(self.queues.present.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queues.present.clone(), self.swapchain.clone(), image_number)
            .then_signal_fence_and_flush();

        match present_future {
            Ok(future) => previous_frame_end = Box::new(future) as Box<_>,
            Err(FlushError::OutOfDate) => {
                println!("ERROR: Swapchain out of date");
                self.recreate_swapchain();
                previous_frame_end = Box::new(sync::now(self.device.clone())) as Box<_>;
            },
            // Why can we continue here?
            Err(err) => {
                println!("{:?}", err);
                previous_frame_end = Box::new(sync::now(self.device.clone())) as Box<_>;
            }
        }

        previous_frame_end
    }

    fn recreate_swapchain(&mut self) -> Result<(), SwapchainCreationError> {
        let dimensions = self.surface.capabilities(self.device.physical_device()).unwrap().current_extent.unwrap_or([1280, 720]);

        let (new_swapchain, new_images) = self.swapchain.recreate_with_dimension(dimensions)?;

        self.dynamic_state.viewports = Some(vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [self.swapchain.dimensions()[0] as f32, self.swapchain.dimensions()[1] as f32],
                depth_range: 0.0 .. 1.0,
            }]);

        self.framebuffers = None;

        mem::replace(&mut self.swapchain, new_swapchain);
        mem::replace(&mut self.images, new_images);

        Ok(())
    }

    fn register_debug_callback(instance: Arc<instance::Instance>) -> Option<DebugCallback> {
        let message_types = MessageTypes {
            error: true,
            warning: true,
            performance_warning: true,
            information: false,
            debug: true,
        };

        DebugCallback::new(&instance, message_types, |msg| {
            println!("Debug callback from {}: {}", msg.layer_prefix, msg.description);
        })
        .ok()
    }

    fn remove_debug_callback(&mut self) {
        self._callback = None;
    }

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
                khr_mir_surface: true,

                // Android
                khr_android_surface: true,

                // Windows
                khr_win32_surface: true,

                // Apple
                mvk_ios_surface: true,
                mvk_macos_surface: true,

                // Debugging
                ext_debug_report: true,

                .. InstanceExtensions::none()
            };

            let supported = InstanceExtensions::supported_by_core().expect("Failed to load supported instance extensions");

            supported.intersection(&desired)
        };

        println!("Requested extensions: {:?}\n", extensions);

        // FIXME Check for with-debugging feature
        let layers = {
            let desired = [
                //"VK_LAYER_LUNARG_api_dump",
                //"VK_LAYER_LUNARG_core_validation",
                //"VK_LAYER_LUNARG_device_simulation",
                //"VK_LAYER_LUNARG_monitor",
                //"VK_LAYER_LUNARG_object_tracker",
                //"VK_LAYER_LUNARG_parameter_validation",
                //"VK_LAYER_LUNARG_screenshot",
                "VK_LAYER_LUNARG_standard_validation",
                //"VK_LAYER_LUNARG_vktrace",
            ];

            for dlayer in desired.clone().iter() {
                let mut available = instance::layers_list().unwrap();

                available.find(|alayer| {
                    alayer.name() == *dlayer
                })
                .expect("Failed to find validation layer");
            }

            desired
        };

        println!("Requested layers: {:?}\n", layers);

        instance::Instance::new(Some(&info), &extensions, layers.iter()).unwrap()
    }

    fn new_device_and_queues(instance: Arc<instance::Instance>, surface: Surface) -> (Arc<Device>, queues::Queues) {
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
                        let queue_family_ids = QueueFamilyIds::from_queue_families_iter(device.queue_families(), surface.clone());

                        (device, score, queue_family_ids)
                    })
                    .inspect(|(device, score, _)| {
                        println!("\
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
                if let Some(id) = queue_family_ids.general {
                    let qf = physical.queue_family_by_id(id).unwrap();

                    println!("{:?}", qf.queues_count());

                    for _ in 0..min(4, qf.queues_count()) {
                        queues.push((qf, 1.0f32));
                        queue_types.push(QueueFamilyTypes::General);
                    }
                }
                else if queues.len() < 1 {
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

            // TODO: Check for minimum required features
            let features = instance::Features::none();

            let required_extensions = DeviceExtensions {
                khr_swapchain: true,
                .. DeviceExtensions::none()
            };

            let device_extensions = DeviceExtensions::supported_by_device(physical).intersection(&required_extensions);

            // Check if requirements are met
            assert_eq!(device_extensions, required_extensions);

            let (device, queues_iter) = Device::new(physical, &features, &device_extensions, queues).expect("Failed to create logical device");

            // FIXME What if there are more then one general queue
            let queues = {
                let queues = queues_iter.collect::<Vec<_>>();

                // All vulkan implementations have to have one general queue
                // Therefor it is ok to panic if we cant get one
                let general = queues[0].clone();

                let compute = if queue_types.get(1) == Some(&QueueFamilyTypes::Compute) || queue_types.get(1) == Some(&QueueFamilyTypes::General) {
                    queues[1].clone()
                }
                else {
                    general.clone()
                };

                let graphics = if queue_types.get(2) == Some(&QueueFamilyTypes::Graphics) || queue_types.get(2) == Some(&QueueFamilyTypes::General) {
                    queues[2].clone()
                }
                else {
                    general.clone()
                };

                let present = if queue_types.get(3) == Some(&QueueFamilyTypes::Present) || queue_types.get(3) == Some(&QueueFamilyTypes::General) {
                    queues[3].clone()
                }
                else {
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

    fn new_swapchain_and_images(device: Arc<Device>, surface: Surface, queues: &queues::Queues) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
            use vulkano::image::ImageUsage;
            use vulkano::sync::SharingMode;
            use vulkano::swapchain::{Swapchain, CompositeAlpha, PresentMode};

            let capabilities = surface.capabilities(device.physical_device()).expect("Failed to get surface capabilities");

            println!("Surface capabilities: {:?}\n", capabilities);

            let buffer_count = max(capabilities.min_image_count, capabilities.max_image_count.unwrap_or(1));

            // First available format
            let format = capabilities.supported_formats[0].0;

            // Current extent seems to be maximized window normaly
            let dimensions = capabilities.current_extent.unwrap_or([1280, 720]);

            // We will only use this image for color
            let image_usage = ImageUsage {
                color_attachment: true,
                .. ImageUsage::none()
            };

            // Only our present queue needs access to this image
            let sharing_mode = SharingMode::Exclusive(queues.present.family().id());

            // We dont need support for flipping the window or anything similar
            let transform = capabilities.current_transform;

            // We prefer a non-transparent window
            let alpha_composite = if capabilities.supported_composite_alpha.supports(CompositeAlpha::Opaque) {
                CompositeAlpha::Opaque
            }
            else if capabilities.supported_composite_alpha.supports(CompositeAlpha::Inherit) {
                CompositeAlpha::Inherit
            }
            else {
                capabilities.supported_composite_alpha.iter().next().unwrap()
            };

            // We prefer Mailbox, then Fifo
            let present_mode = if capabilities.present_modes.supports(PresentMode::Mailbox) {
                PresentMode::Mailbox
            }
            else if capabilities.present_modes.supports(PresentMode::Fifo) {
                PresentMode::Fifo
            }
            else {
                capabilities.present_modes.iter().next().unwrap()
            };

            // First available color space for our format
            //let color_space = capabilities.supported_formats[0].1;

            Swapchain::new(device.clone(),
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
                           None)
                .expect("Failed to create swapchain")
    }

    fn load_shaders(device: Arc<Device>) -> ShaderSet {
        let vertex = shaders::vertex::Shader::load(device.clone()).expect("Failed to create shader module");
        let fragment = shaders::fragment::Shader::load(device.clone()).expect("Failed to create shader module");

        ShaderSet {
            vertex,
            fragment,
        }
    }

    fn build_render_pass(device: Arc<Device>, format: Format) -> Arc<RenderPassAbstract + Send + Sync> {
        Arc::new(single_pass_renderpass!(device.clone(),
            attachments: {
                // `color` is a custom name
                color: {
                    load: Clear,
                    store: Store,
                    format: format,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap())
    }

    fn build_graphics_pipeline(device: Arc<Device>, render_pass: Arc<RenderPassAbstract + Send + Sync>, shaders: &ShaderSet) -> Arc<GraphicsPipelineAbstract + Send + Sync> {
        let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(shaders.vertex.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(shaders.fragment.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass, 0).unwrap())
            .build(device.clone())
            .unwrap());

        pipeline
    }
}
