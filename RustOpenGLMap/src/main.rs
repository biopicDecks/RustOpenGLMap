extern crate gl;
mod opengl_helper;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{self, GLContext};
type Vertex = [f32; 3];
type TriIndexes = [u32; 3];
const VERTICES: [Vertex; 4] = [
    [0.5, 0.5, 0.0],
    [0.5, -0.5, 0.0],
    [-0.5, -0.5, 0.0],
    [-0.5, 0.5, 0.0],
];
const INDICES: [TriIndexes; 2] = [[0, 1, 3], [1, 2, 3]];

const VERT_SHADER: &str = r#"#version 410 core
  layout (location = 0) in vec3 pos;
  void main() {
    gl_Position = vec4(pos.x, pos.y, pos.z, 1.0);
  }
"#;

const FRAG_SHADER: &str = r#"#version 410 core
  out vec4 final_color;

  void main() {
    final_color = vec4(1.0, 0.5, 0.2, 1.0);
  }
"#;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    video_subsystem.gl_attr().set_context_major_version(4);
    video_subsystem.gl_attr().set_context_minor_version(1);
    video_subsystem
        .gl_attr()
        .set_context_profile(video::GLProfile::Core);

    let window = video_subsystem
        .window("MapWindow", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let _gl_context: GLContext = window.gl_create_context()?;
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const _);

    println!("Hello, world!");

    let mut event_pump = sdl_context.event_pump()?;

    // compile vertex shader

    unsafe {
        // Now calls like gl::ClearColor should be recognized
        gl::ClearColor(0.7, 0.1, 0.5, 1.0);
        // gl::COLOR_BUFFER_BIT comes from gl::types::GLenum

        let vao = opengl_helper::VertexArray::new().expect("Couldn't make a VAO");
        vao.bind();
        let vbo = opengl_helper::Buffer::new().expect("Couldn't make a VBO");
        vbo.bind(opengl_helper::BufferType::Array);
        opengl_helper::Buffer::data(
            opengl_helper::BufferType::Array,
            bytemuck::cast_slice(&VERTICES),
            gl::STATIC_DRAW,
        );

        let ebo = opengl_helper::Buffer::new().expect("Couldn't make the element buffer.");
        ebo.bind(opengl_helper::BufferType::ElementArray);
        opengl_helper::Buffer::data(
            opengl_helper::BufferType::ElementArray,
            bytemuck::cast_slice(&INDICES),
            gl::STATIC_DRAW,
        );

        let p = opengl_helper::ShaderProgram::from_vert_frag(VERT_SHADER, FRAG_SHADER)?;

        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>().try_into().unwrap(),
            0 as *const _,
        );
        gl::EnableVertexAttribArray(0);
    }

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
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
        }
        window.gl_swap_window();
        ::std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}
