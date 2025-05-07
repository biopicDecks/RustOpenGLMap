extern crate gl;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::{self, GLContext};

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    video_subsystem.gl_attr().set_context_major_version(4);
    video_subsystem.gl_attr().set_context_minor_version(1);
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

    type Vertex = [f32; 3];
    const VERTICES: [Vertex; 3] =
        [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]];

    
 // compile vertex shader
    
    unsafe {
        let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
        assert_ne!(vertex_shader, 0);

        const VERT_SHADER: &str = r#"#version 410 core
  layout (location = 0) in vec3 pos;
  void main() {
    gl_Position = vec4(pos.x, pos.y, pos.z, 1.0);
  }
"#;
        
        gl::ShaderSource(
            vertex_shader,
            1,
            &(VERT_SHADER.as_bytes().as_ptr().cast()),
            &(VERT_SHADER.len().try_into().unwrap()),
        );

        gl::CompileShader(vertex_shader);

        let mut success = 0;
        gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);

        if success == 0 {
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            let mut log_len = 0_i32;
            gl::GetShaderInfoLog(
                vertex_shader,
                1024,
                &mut log_len,
                v.as_mut_ptr().cast(),
            );
            v.set_len(log_len.try_into().unwrap());
            panic!("Vertex Compile Error: {}", String::from_utf8_lossy(&v));
        }
        else { 
            println!("Vertex Shader Compiled Succccesfully");
        } 

    // compile fragment shader
        
        let frag_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        assert_ne!(frag_shader, 0);

        const FRAG_SHADER: &str = r#"#version 410 core
  out vec4 final_color;

  void main() {
    final_color = vec4(1.0, 0.5, 0.2, 1.0);
  }
"#;

        gl::ShaderSource(
            frag_shader,
            1,
            &(FRAG_SHADER.as_bytes().as_ptr().cast()),
            &(FRAG_SHADER.len().try_into().unwrap()),
        );

        gl::CompileShader(frag_shader);

        success = 0;
        gl::GetShaderiv(frag_shader, gl::COMPILE_STATUS, &mut success);

        if success == 0 {
            let mut v: Vec<u8> = Vec::with_capacity(1024);
            let mut log_len = 0_i32;
            gl::GetShaderInfoLog(
                frag_shader,
                1024,
                &mut log_len,
                v.as_mut_ptr().cast(),
            );
            v.set_len(log_len.try_into().unwrap());
            panic!("Fragment Compile Error: {}", String::from_utf8_lossy(&v));
        }
        else {
            println!("Fragment Shader Compiled Succccesfully");
        }
        
        let shader_program = gl::CreateProgram();
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, frag_shader);
        gl::LinkProgram(shader_program);
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
            // Now calls like gl::ClearColor should be recognized
            gl::ClearColor(0.2, 0.6, 0.5, 1.0);
            // gl::COLOR_BUFFER_BIT comes from gl::types::GLenum
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        unsafe {
            let mut vao = 0;
            gl::GenVertexArrays(1, &mut vao);
            assert_ne!(vao, 0);
        }

        unsafe {
            let mut vbo = 0;
            gl::GenBuffers(1, &mut vbo);
            assert_ne!(vbo, 0);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            
            gl::BufferData(
                gl::ARRAY_BUFFER,
                size_of_val(&VERTICES) as isize,
                VERTICES.as_ptr().cast(),
                gl::STATIC_DRAW,
            );
        }

        unsafe {
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
        
        window.gl_swap_window();

        ::std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())

}

