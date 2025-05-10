extern crate gl;
mod opengl_helper;

use image::DynamicImage;
use image::ImageReader;
use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{self, GLContext};

type Vertex = [f32; 3 + 3 + 2];
type TriIndexes = [u32; 3];
const VERTICES: [Vertex; 4] = [
    // top right
    [0.5, 0.5, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0],
    // bottom right
    [0.5, -0.5, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0],
    // bottom left
    [-0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    // top left
    [-0.5, 0.5, 0.0, 0.2, 0.3, 0.4, 0.0, 1.0],
];

const INDICES: [TriIndexes; 2] = [[0, 1, 3], [1, 2, 3]];

const VERT_SHADER: &str = r#"#version 410 core
  layout (location = 0) in vec3 pos;
  layout (location = 1) in vec3 color;
  layout (location = 2) in vec2 tex;

  out vec4 frag_color;
  out vec2 frag_tex;

  void main() {
    gl_Position = vec4(pos, 1.0);
    frag_color = vec4(color, 1.0);
    frag_tex = tex;
  }
"#;

const FRAG_SHADER: &str = r#"#version 410 core
  uniform sampler2D the_texture;

  in vec4 frag_color;
  in vec2 frag_tex;

  out vec4 final_color;

  void main() {
    final_color = texture(the_texture, frag_tex);
  }
"#;

fn main() -> Result<(), String> {
    let bitmap = {
        let img: DynamicImage = ImageReader::open("test.png")
            .expect("Failed to open image")
            .decode()
            .expect("Failed to decode image");

        let mut rgba_image = img.to_rgba8();

        // Flip scanlines (vertical flip) if needed
        image::imageops::flip_vertical_in_place(&mut rgba_image);

        rgba_image
    };

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
    }
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

    let _ = opengl_helper::create_texture_from_bitmap(&bitmap);

    let _ = opengl_helper::ShaderProgram::from_vert_frag(VERT_SHADER, FRAG_SHADER)?;
    unsafe {
        // position
        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>().try_into().unwrap(),
            0 as *const _,
        );
        gl::EnableVertexAttribArray(0);

        // colour
        gl::VertexAttribPointer(
            1,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>().try_into().unwrap(),
            size_of::<[f32; 3]>() as *const _,
        );
        gl::EnableVertexAttribArray(1);

        // tex
        gl::VertexAttribPointer(
            2,
            2,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>().try_into().unwrap(),
            size_of::<[f32; 6]>() as *const _,
        );
        gl::EnableVertexAttribArray(2);

        opengl_helper::polygon_mode(opengl_helper::PolygonMode::Fill);
    }

    //let c_str = CString::new("uni_color").unwrap();
    //let p: *const c_char = c_str.as_ptr();
    //let uni_color_loc = unsafe { gl::GetUniformLocation(shader_program.0, p) };
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
        //let time = sdl_context.timer()?.ticks() as f32 / 1000.0_f32;
        //let green = (f32::sin(time) / 2.0) + 0.5;
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
            //gl::Uniform4f(uni_color_loc, 0.1, green, 0.1, 1.0);
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, 0 as *const _);
        }
        window.gl_swap_window();
        ::std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}
