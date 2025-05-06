extern crate gl;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{self, GLContext};

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    video_subsystem.gl_attr().set_context_major_version(3);
    video_subsystem.gl_attr().set_context_minor_version(3);
    video_subsystem.gl_attr().set_context_profile(video::GLProfile::Core);


    let window = video_subsystem
        .window("MapWindow", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let _gl_context: GLContext = window.gl_create_context()?;
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);


    println!("Hello, world!");

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        unsafe {
            // Now calls like gl::ClearColor should be recognized
            gl::ClearColor(0.2, 0.6, 0.5, 1.0);
            // gl::COLOR_BUFFER_BIT comes from gl::types::GLenum
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        window.gl_swap_window();

        ::std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())

}

