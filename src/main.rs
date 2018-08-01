extern crate winit;
#[macro_use]
extern crate vulkano;
//extern crate vulkano_win;
#[macro_use]
extern crate vulkano_shader_derive;

mod vulkano_win;

use vulkano_win::VkSurfaceBuild;

use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::command_buffer::{DynamicState, AutoCommandBufferBuilder};
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::instance;
use vulkano::instance::{PhysicalDevice, PhysicalDeviceType, DeviceExtensions, InstanceExtensions};
use vulkano::instance::debug::{DebugCallback, MessageTypes};
use vulkano::device::Device;
use vulkano::sync;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::pipeline::{GraphicsPipeline, viewport::Viewport};
use vulkano::swapchain;
use vulkano::swapchain::{AcquireError, SwapchainCreationError};

use std::cmp::{min, max};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct QueueFamilyIds {
    pub general: Option<u32>,
    pub graphics: Option<u32>,
    pub compute: Option<u32>,
    pub transfer: Option<u32>,
}

impl QueueFamilyIds {
    pub fn none() -> Self {
        Self {
            general: None,
            graphics: None,
            compute: None,
            transfer: None,
        }
    }
}

impl Iterator for QueueFamilyIds {
    type Item = u32;

    // TODO Find a better way to do all this
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ret) = self.general {
            self.general = None;
            return Some(ret);
        }
        if let Some(ret) = self.graphics {
            self.graphics = None;
            return Some(ret);
        }
        if let Some(ret) = self.compute {
            self.compute = None;
            return Some(ret);
        }
        if let Some(ret) = self.transfer {
            self.transfer = None;
            return Some(ret);
        }
        None
    }
}

// TODO Put everything in a struct
fn main() {
    let mut events_loop = winit::EventsLoop::new();

    let (device, mut qiter, surface, _callback) = {
        let instance = {
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

                    // Debuging
                    ext_debug_report: true,


                    .. InstanceExtensions::none()
                };

                let supported = InstanceExtensions::supported_by_core().expect("Failed to load supported instance extensions");

                supported.intersection(&desired)
            };

            println!("Requested extensions: {:?}\n", extensions);

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
        };

        let _callback = {
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
        };

        let surface = winit::WindowBuilder::new()
            .with_title("VK Engine")
            .build_vk_surface(&events_loop, instance.clone())
            .unwrap();

        let (physical, mut queue_family_ids) = {
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
                    // For Nvidia gpus we can expect 16 general queues in one queue family
                    // For Intel gpus we can expect 1 general queue
                    // For AMD gpus
                    let queue_family_ids = {
                        let mut queue_family_ids = QueueFamilyIds::none();

                        // Find a general queue family
                        // A general queue family supports all operations
                        queue_family_ids.general = device.queue_families()
                            .enumerate()
                            .find(|(_, qf)| {
                                qf.supports_transfers() &&
                                qf.supports_compute() &&
                                qf.supports_graphics() &&
                                surface.is_supported(*qf).unwrap_or(false)
                            })
                            .map(|(id, _)| {
                                id as u32
                            });

                        // Find queue family that only supports transfers
                        queue_family_ids.transfer = device.queue_families()
                            .enumerate()
                            .find(|(_, qf)| {
                                qf.supports_transfers() &&
                                !qf.supports_compute() &&
                                !qf.supports_graphics()
                            })
                            .map(|(id, _)| {
                                id as u32
                            });

                        // Find queue family that only supports compute
                        queue_family_ids.compute = device.queue_families()
                            .enumerate()
                            .find(|(_, qf)| {
                                qf.supports_compute() &&
                                !qf.supports_graphics()
                            })
                            .map(|(id, _)| {
                                id as u32
                            });

                        // Find queue family that only supports graphics
                        // The graphics queue family also has to support presenting to the surface
                        queue_family_ids.graphics = device.queue_families()
                            .enumerate()
                            .find(|(_, qf)| {
                                qf.supports_graphics() &&
                                surface.is_supported(*qf).unwrap_or(false) &&
                                !qf.supports_compute()
                            })
                            .map(|(id, _)| {
                                id as u32
                            });

                        println!("Queue families: {:?}", queue_family_ids);

                        queue_family_ids
                    };

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

        let mut queues = Vec::with_capacity(physical.queue_families().len());

        // TODO Implement support for using multiple queues
        if let Some(id) = queue_family_ids.general {
            let qf = physical.queue_family_by_id(id).unwrap();

            for _ in 0..min(3, qf.queues_count()) {
                queues.push((qf, 1.0f32));
            }
        }

        println!("Queues to be created: {:?}", queues.len());

        // TODO: Check for minimum required features
        let features = instance::Features::none();

        let required_extensions = DeviceExtensions {
            khr_swapchain: true,
            .. DeviceExtensions::none()
        };

        let device_extensions = DeviceExtensions::supported_by_device(physical).intersection(&required_extensions);

        // Check if requirements are met
        assert_eq!(device_extensions, required_extensions);

        let (device, qiter) = Device::new(physical, &features, &device_extensions, queues).expect("Failed to create logical device");

        (device, qiter, surface, _callback)
    };

    let (mut swapchain, mut images) = {
        use vulkano::image::ImageUsage;
        use vulkano::swapchain::{Swapchain, CompositeAlpha, PresentMode};

        let capabilities = surface.capabilities(device.physical_device()).expect("Failed to get surface capabilities");

        let buffer_count = max(capabilities.min_image_count, capabilities.max_image_count.unwrap_or(1));

        // First available format
        let format = capabilities.supported_formats[0].0;

        // Dimensions of the surface should match the inner size of the window
        let dimensions = capabilities.current_extent.unwrap_or([1280, 720]);

        // We will only use this image for color
        let image_usage = ImageUsage {
            color_attachment: true,
            .. ImageUsage::none()
        };

        // Only our present queue needs access to this image
        let sharing_mode = vulkano::sync::SharingMode::Exclusive(qiter.next().unwrap().family().id());

        let transform = capabilities.current_transform;

        // We prefer a non-transparent window
        let alpha_composite = if capabilities.supported_composite_alpha.supports(CompositeAlpha::Inherit) {
            CompositeAlpha::Inherit
        } else if capabilities.supported_composite_alpha.supports(CompositeAlpha::Opaque) {
            CompositeAlpha::Opaque
        } else {
            capabilities.supported_composite_alpha.iter().next().unwrap()
        };

        // We prefer Mailbox, but we will take what we get
        let present_mode = if capabilities.present_modes.supports(PresentMode::Mailbox) {
            PresentMode::Mailbox
        } else {
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
    };

    let vertex_buffer = {
        #[derive(Debug, Clone)]
        struct Vertex { position: [f32; 2] }
        impl_vertex!(Vertex, position);

        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), [
            Vertex { position: [-0.5, -0.25] },
            Vertex { position: [0.0, 0.5] },
            Vertex { position: [0.25, -0.1] }
        ].iter().cloned()).expect("failed to create buffer")
    };

    mod vs {
        #[derive(VulkanoShader)]
        #[ty = "vertex"]
        #[src = "
        #version 450

        layout(location = 0) in vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
        "]
            struct Dummy;
    }

    mod fs {
        #[derive(VulkanoShader)]
        #[ty = "fragment"]
        #[src = "
        #version 450

        layout(location = 0) out vec4 f_color;

        void main() {
            f_color = vec4(1.0, 0.0, 0.0, 1.0);
        }
        "]
            struct Dummy;
    }

    let vs = vs::Shader::load(device.clone()).expect("failed to create shader module");
    let fs = fs::Shader::load(device.clone()).expect("failed to create shader module");

    let render_pass = Arc::new(single_pass_renderpass!(device.clone(),
        attachments: {
            // `color` is a custom name
            color: {
                load: Clear,
                store: Store,
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {}
        }
    ).unwrap());

    let pipeline = Arc::new(GraphicsPipeline::start()
        .vertex_input_single_buffer()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap());

    let mut framebuffers: Option<Vec<Arc<vulkano::framebuffer::Framebuffer<_,_>>>> = None;

    let mut swapchain_invalid = false;
    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<GpuFuture>;

    println!("Queue Iter: {:?}", qiter);
    // This assumes the last queue supports graphics
    let present_queue = qiter.last().expect("Failed to get queue");
    println!("Queue: {:?}", present_queue);

    'gameloop: loop {
        previous_frame_end.cleanup_finished();

        if swapchain_invalid {
            let dimensions = surface.capabilities(device.physical_device()).unwrap().current_extent.unwrap_or([1280, 720]);

            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(new) => new,
                // Can happen if the user is resizing the window
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err),
            };

            std::mem::replace(&mut swapchain, new_swapchain);
            std::mem::replace(&mut images, new_images);

            swapchain_invalid = false;
        }

        if framebuffers.is_none() {
            let new_framebuffers = Some(images.iter().map(|image| {
                Arc::new(Framebuffer::start(render_pass.clone())
                         .add(image.clone()).unwrap()
                         .build().unwrap())
            }).collect::<Vec<_>>());
            std::mem::replace(&mut framebuffers, new_framebuffers);
}

        let (image_number, acquired_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
            Ok(ret) => ret,
            // Can happen if the user has resized the window
            Err(AcquireError::OutOfDate) => {
                println!("ERROR: Swapchain out of date");
                swapchain_invalid = true;
                continue;
            },
            Err(err) => panic!("Error occurred while acquiring next image: {:?}", err),
        };

        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), present_queue.family()).unwrap()
        // Before we can draw, we have to *enter a render pass*. There are two methods to do
        // this: `draw_inline` and `draw_secondary`. The latter is a bit more advanced and is
        // not covered here.
        //
        // The third parameter builds the list of values to clear the attachments with. The API
        // is similar to the list of attachments when building the framebuffers, except that
        // only the attachments that use `load: Clear` appear in the list.
        .begin_render_pass(framebuffers.as_ref().unwrap()[image_number].clone(), false,
                           vec![[0.0, 0.0, 1.0, 1.0].into()])
        .unwrap()

        // We are now inside the first subpass of the render pass. We add a draw command.
        //
        // The last two parameters contain the list of resources to pass to the shaders.
        // Since we used an `EmptyPipeline` object, the objects have to be `()`.
        .draw(pipeline.clone(),
              DynamicState {
                  line_width: None,
                  // TODO: Find a way to do this without having to dynamically allocate a Vec every frame.
                  viewports: Some(vec![Viewport {
                      origin: [0.0, 0.0],
                      dimensions: [swapchain.dimensions()[0] as f32, swapchain.dimensions()[1] as f32],
                      depth_range: 0.0 .. 1.0,
                  }]),
                  scissors: None,
              },
              vertex_buffer.clone(), (), ())
        .unwrap()

        // We leave the render pass by calling `draw_end`. Note that if we had multiple
        // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
        // next subpass.
        .end_render_pass()
        .unwrap()

        // Finish building the command buffer by calling `build`.
        .build()
        .unwrap();

        let present_future = previous_frame_end.join(acquired_future)
            .then_execute(present_queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(present_queue.clone(), swapchain.clone(), image_number)
            .then_signal_fence_and_flush();

        match present_future {
            Ok(future) => previous_frame_end = Box::new(future) as Box<_>,
            Err(FlushError::OutOfDate) => {
                println!("ERROR: Swapchain out of date");
                swapchain_invalid = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            },
            // Why can we continue here?
            Err(err) => {
                println!("{:?}", err);
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
        }

        let mut should_close = false;
        events_loop.poll_events(|event| {
            use winit::{
                Event::WindowEvent as Window,
                WindowEvent,
            };

            match event {
                Window { event: WindowEvent::CloseRequested, .. } => should_close = true,
                _ => (),
            }
        });

        if should_close {
            break 'gameloop;
        }

        // TODO Implement/enable vsync
        std::thread::sleep(std::time::Duration::from_millis(16u64));
    }

}
