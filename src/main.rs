// extern crate winit;
#[macro_use]
extern crate vulkano;
// extern crate vulkano_win;
// #[macro_use]
// extern crate vulkano_shader_derive;

use vulkano::instance;
use vulkano::instance::{PhysicalDevice, PhysicalDeviceType};
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

            //let mut devices: Vec<(PhysicalDevice, u32)> = physical_devices.zip(physical_devices_scores)
            let mut devices = PhysicalDevice::enumerate(&instance)
                .map(|device| {
                    let mut score = 0u32;

                    // Score for device type
                    match device.ty() {
                        PhysicalDeviceType::DiscreteGpu => score += 10_000u32,
                        PhysicalDeviceType::IntegratedGpu => score += 5_000u32,
                        _ => (),
                    }

                    // Score for device api version
                    let ver = device.api_version();
                    score += (ver.major * 1_000) as u32;
                    score += (ver.minor * 100) as u32;
                    score += (ver.patch * 2) as u32;

                    // Query for graphics and compute support
                    {
                        let support_graphics = device.queue_families()
                            .filter(|qf| {
                                qf.supports_graphics()
                            })
                            .next()
                            .is_some();

                        let support_compute = device.queue_families()
                            .filter(|qf| {
                                qf.supports_compute()
                            })
                            .next()
                            .is_some();

                        // Graphics and compute are hard reqs
                        if !support_graphics || !support_compute {
                            score = 0;
                        };
                    }

                    (device, score)
                })
                .inspect(|(device, score)| {
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
            devices.sort_by(|(_, a), (_, b)| b.cmp(&a));

            let (physical, score) = devices.get(0).unwrap().clone();
            assert_ne!(score, 0u32);

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
