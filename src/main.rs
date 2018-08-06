extern crate winit;
#[macro_use]
extern crate vulkano;
//extern crate vulkano_win;
#[macro_use]
extern crate vulkano_shader_derive;

mod renderer;

use vulkano::sync;

fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let mut renderer = renderer::Renderer::new(&events_loop);

    let mut previous_frame_end = Box::new(sync::now(renderer.device.clone())) as Box<sync::GpuFuture>;

    'gameloop: loop {
        previous_frame_end = renderer.render(previous_frame_end);

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
