// extern crate winit;
#[macro_use]
extern crate vulkano;
// extern crate vulkano_win;
// #[macro_use]
// extern crate vulkano_shader_derive;

use vulkano::instance;
use vulkano::device::Device;

fn main() {
    let (device, qiter) = {
        let instance = {
            let info = app_info_from_cargo_toml!();

            // TODO: Use intersection between supported and desired extensions
            let extensions = vulkano::instance::InstanceExtensions::supported_by_core().expect("Failed to load supported instance extensions");

            //let layers = &instance::layers_list().unwrap();

            instance::Instance::new(Some(&info), &extensions, None).unwrap()
        };

        let physical = {
            println!("Listing enumerated devices...\n");

            // Get all discrete gpus that supprort graphics
            let mut physical_devices = instance::PhysicalDevice::enumerate(&instance)
                // Print the gpus
                .inspect(|d| {
                    let mut device_info = String::new();

                    device_info += &format!("Device name: {}\n", d.name());
                    device_info += &format!("Device type: {:?}\n", d.ty());
                    device_info += &format!("Device api version: {:?}\n", d.api_version());

                    println!("{}", device_info);
                })
                // We only want discrete gpus
                .filter(|d| d.ty() == instance::PhysicalDeviceType::DiscreteGpu)
                // We only want gpus that support graphics
                .filter(|d| d.queue_families().filter(|qf| qf.supports_graphics()).next().is_some())
                .collect::<Vec<_>>();

            // Sort them by vulkan version
            physical_devices.sort_by(|a, b| a.api_version().into_vulkan_version().cmp(&b.api_version().into_vulkan_version()));

            // Choose the discrete gpu with the highest vulkan version
            let physical = physical_devices.get(0).expect("No discrete gpus supports graphics").clone();

            physical
        };

        let qf = physical.queue_families().filter(|qf| qf.supports_graphics()).next().unwrap();

        // TODO: Check for minimum required features
        let features = instance::Features::none();

        // TODO: Use intersection between supported and desired extensions
        let device_extensions = instance::DeviceExtensions::supported_by_device(physical);

        Device::new(physical, &features, &device_extensions, Some((qf, 1.0))).expect("Failed to create logical device")
    };
}
