extern crate winit;
#[macro_use]
extern crate vulkano;
// extern crate vulkano_win;
// #[macro_use]
// extern crate vulkano_shader_derive;

mod vulkano_win;

use vulkano_win::VkSurfaceBuild;

use vulkano::instance;
use vulkano::instance::{PhysicalDevice, PhysicalDeviceType, DeviceExtensions, QueueFamily};
use vulkano::device::Device;

#[derive(Debug, Clone)]
struct QueueFamilyIds {
    pub transfer: Option<usize>,
    pub compute: Option<usize>,
    pub graphics: Option<usize>,
    pub present: Option<usize>,
}

impl QueueFamilyIds {
    pub fn none() -> Self {
        Self {
            transfer: None,
            compute: None,
            graphics: None,
            present: None,
        }
    }
}

impl Iterator for QueueFamilyIds {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ret) = self.transfer {
            self.transfer = None;
            return Some(ret);
        }
        if let Some(ret) = self.compute {
            self.compute = None;
            return Some(ret);
        }
        if let Some(ret) = self.graphics {
            self.graphics = None;
            return Some(ret);
        }
        if let Some(ret) = self.present {
            self.present = None;
            return Some(ret);
        }
        None
    }
}

fn main() {
    let mut events_loop = winit::EventsLoop::new();

    let (device, mut qiter, surface) = {
        let instance = {
            let info = app_info_from_cargo_toml!();

            // TODO: Use intersection between supported and desired extensions
            let extensions = vulkano::instance::InstanceExtensions::supported_by_core().expect("Failed to load supported instance extensions");

            //let layers = &instance::layers_list().unwrap();

            instance::Instance::new(Some(&info), &extensions, None).unwrap()
        };

        let surface = winit::WindowBuilder::new()
            .build_vk_surface(&events_loop, instance.clone())
            .unwrap();

        let (physical, mut queue_family_ids) = {
            println!("Listing enumerated devices...\n");

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
                    let ver = device.api_version();
                    score += (ver.major * 1_000) as u32;
                    score += (ver.minor * 100) as u32;
                    score += (ver.patch * 2) as u32;

                    // Stores the ids for the queue families we want to use
                    // We assume that there is only one queue family for each operation (this is true for most vulkan implementations)
                    let queue_family_ids = {
                        let mut queue_family_ids = QueueFamilyIds::none();

                        // Find queue family that only supports transfers
                        queue_family_ids.transfer = device.queue_families()
                            .enumerate()
                            .find(|(id, qf)| {
                                qf.supports_transfers() && !qf.supports_compute() || !qf.supports_graphics()
                            })
                            .map(|(id, _)| {
                                id
                            });

                        // Find queue family that only supports compute
                        queue_family_ids.compute = device.queue_families()
                            .enumerate()
                            .find(|(id, qf)| {
                                qf.supports_compute() && !qf.supports_graphics()
                            })
                            .map(|(id, _)| {
                                id
                            });

                        // Find queue family that can present to our surface, but is not our graphics queue family
                        queue_family_ids.present = device.queue_families()
                            .enumerate()
                            .find(|(id, qf)| {
                                surface.is_supported(*qf).unwrap_or(false) && !qf.supports_graphics()
                            })
                            .map(|(id, _)| {
                                id
                            });

                        // Find queue family that supports graphics
                        // The graphics queue family will have to present if we do not have a dedicated present queue family
                        queue_family_ids.graphics = device.queue_families()
                            .enumerate()
                            .find(|(id, qf)| {
                                if queue_family_ids.present == None {
                                    qf.supports_graphics() && surface.is_supported(*qf).unwrap_or(false)
                                } else {
                                    qf.supports_graphics()
                                }
                            })
                            .map(|(id, _)| {
                                id
                            });

                        let queue_family_count = queue_family_ids.clone().count();
                        score += queue_family_count as u32 * 100;

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

            // Sort them by score
            devices.sort_by(|(_, a, _), (_, b, _)| a.cmp(&b));

            let (physical, score, queue_family_ids) = devices.pop().unwrap();
            assert_ne!(score, 0u32);

            (physical, queue_family_ids)
        };

        // We only care about the graphics queue family for now
        let qf = queue_family_ids.graphics.unwrap();
        let qf = physical.queue_family_by_id(qf as u32).unwrap();

        let queue_families = vec!((qf, 1.0f32));

        // TODO: Check for minimum required features
        let features = instance::Features::none();

        let required_extensions = DeviceExtensions {
            khr_swapchain: true,
            .. DeviceExtensions::none()
        };

        let device_extensions = DeviceExtensions::supported_by_device(physical).intersection(&required_extensions);

        // Check if requirements are met
        assert_eq!(device_extensions, required_extensions);

        let (device, qiter) = Device::new(physical, &features, &device_extensions, queue_families).expect("Failed to create logical device");

        (device, qiter, surface)
    };

    let (mut swapchain, mut images) = {
        use std::cmp::max;
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
        let color_space = capabilities.supported_formats[0].1;

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

    std::thread::sleep_ms(2000);
}
