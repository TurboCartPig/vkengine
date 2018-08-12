extern crate winit;
#[macro_use]
extern crate vulkano;
extern crate vulkano_win;
#[macro_use]
extern crate vulkano_shader_derive;
extern crate vulkano_text;
extern crate float_duration;

mod renderer;

use vulkano::sync;

use float_duration::{FloatDuration, TimePoint};

use std::time::Instant;

//TODO Use a logger instead of println
//TODO Mesh loading

fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let mut renderer = renderer::Renderer::new(&events_loop);

    let mut previous_frame_end = Box::new(sync::now(renderer.device.clone())) as Box<sync::GpuFuture>;
    let mut frame_time = FloatDuration::seconds(1f64); // Cant ever be zero

    'gameloop: loop {
        let frame_start_time = Instant::now();

        previous_frame_end = renderer.render(previous_frame_end, frame_time);

        let mut should_close = false;
        events_loop.poll_events(|event| {
            use winit::Event::WindowEvent as Window;
            use winit::Event::DeviceEvent as Device;
            use winit::{WindowEvent, DeviceEvent, KeyboardInput};

            match event {
                Window { event: WindowEvent::CloseRequested, .. } => should_close = true,
                Device { event: DeviceEvent::Key(KeyboardInput { scancode: 0x10, .. }), .. } => should_close = true,
                _ => (),
            }
        });

        if should_close {
            break 'gameloop;
        }

        let frame_end_time = Instant::now();
        frame_time = frame_end_time.float_duration_since(frame_start_time).unwrap_or(frame_time);
    }
}
