extern crate gl;
mod opengl_helper;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
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

#[derive(Debug)]
struct TilePos {
    z: u32,
    x: u32,
    y: u32,
}

impl TilePos {
    fn new() -> Self {
        Self { z: 0, x: 0, y: 0 }
    }

    /// Return the child tile that lies under (u,v) in [0,1]².
    /// SDL’s Y axis grows downward, OSM’s Y grows *down*, too,
    /// so no extra flip is needed.
    fn zoom_in(&mut self, u: f64, v: f64) {
        if self.z >= 19 {
            return;
        } // OSM max
        self.x = self.x * 2 + if u >= 0.5 { 1 } else { 0 };
        self.y = self.y * 2 + if v >= 0.5 { 1 } else { 0 };
        self.z += 1;
    }
}

fn main() -> Result<(), String> {
    //let bitmap1 = opengl_helper::load_image("test.png");
    //let bitmap2 = opengl_helper::load_image("test1.png");
    //let mut current_bitmap = &bitmap1;

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

    let mut tile = TilePos::new();

    let bitmap1 = opengl_helper::fetch_tile(tile.z, tile.x, tile.y).unwrap_or_else(|e| {
        eprintln!(
            "Failed to fetch tile {}/{}/{}: {}",
            tile.z, tile.x, tile.y, e
        );
        opengl_helper::load_image("test.png") // your own function returning RgbaImage
    });

    let _ = opengl_helper::create_texture_from_bitmap(&bitmap1);

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
                } => {
                    break 'running;
                }
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    clicks: clicks_in_event,
                    x,
                    y,
                    ..
                } if clicks_in_event >= 2 => {
                    let (w, h) = window.size(); // u32s
                    let u = x as f64 / w as f64; // 0.0 … 1.0
                    let v = y as f64 / h as f64;

                    tile.zoom_in(u, v);
                    println!("Zoomed to {:?}", tile);

                    // fetch & upload the new tile, re‑using the same texture object
                    let bitmap =
                        opengl_helper::fetch_tile(tile.z, tile.x, tile.y).unwrap_or_else(|e| {
                            eprintln!("Tile error: {e}");
                            opengl_helper::load_image("test.png")
                        });

                    let _ = opengl_helper::create_texture_from_bitmap(&bitmap);
                    // └── you just need a helper that binds `texture`, calls glTexSubImage2D or glTexImage2D.
                }

                // | Event::KeyDown {
                //     keycode: Some(Keycode::Up),
                //     ..
                // } => {
                //     zoom_level += 1;
                //     zoom_level = std::cmp::min(zoom_level,19);
                //     let bitmap = opengl_helper::fetch_tile(zoom_level as u32, 0, 0).unwrap_or_else(|e| {
                //         eprintln!("Failed to fetch tile {}/{}/{}: {}", zoom_level, 0, 0, e);
                //         opengl_helper::load_image("test.png")        // your own function returning RgbaImage
                //     });
                //     let _ =opengl_helper::create_texture_from_bitmap(&bitmap);
                //
                //     // `bitmap` dropped here, memory freed
                // }
                // | Event::KeyDown {
                //     keycode: Some(Keycode::Down),
                //     ..
                // } => {
                //     zoom_level -= 1;
                //     zoom_level = std::cmp::max(zoom_level,0);
                //     let bitmap = opengl_helper::fetch_tile(zoom_level as u32, 0, 0).unwrap_or_else(|e| {
                //         eprintln!("Failed to fetch tile {}/{}/{}: {}", zoom_level, 0, 0, e);
                //         opengl_helper::load_image("test.png")        // your own function returning RgbaImage
                //     });
                //     let _ =opengl_helper::create_texture_from_bitmap(&bitmap);
                //
                //     // `bitmap` dropped here, memory freed
                // }
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
