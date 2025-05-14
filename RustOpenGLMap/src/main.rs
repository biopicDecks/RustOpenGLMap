extern crate gl;
mod opengl_helper;
mod tile;
mod viewport;

use std::sync::mpsc::{Receiver, Sender, channel};
// Added for channels
use std::thread;

use lru::LruCache;
use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::video::{self, GLContext};
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tile::TileLoad;
use tile::TilePos;
use viewport::Viewport;

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
layout (location = 2) in vec2 tex;

uniform vec2 u_scale;   // tile-size in NDC
uniform vec2 u_offset;  // per-tile translation in NDC

out vec2 v_tex;

void main() {
    vec2 scaled     = pos.xy * u_scale;
    vec2 translated = scaled  + u_offset;
    gl_Position = vec4(translated, pos.z, 1.0);
    v_tex       = tex;
}

"#;

const FRAG_SHADER: &str = r#"#version 410 core
uniform sampler2D the_texture;
in  vec2 v_tex;
out vec4 final_color;
void main() { final_color = texture(the_texture, v_tex); }
"#;

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

    let shader_program = opengl_helper::ShaderProgram::from_vert_frag(VERT_SHADER, FRAG_SHADER)?;
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
    let mut map = 0;

    // let mut tile = TilePos::new();
    // let bitmap1 = opengl_helper::fetch_tile(tile.z, tile.x, tile.y, map).unwrap_or_else(|e| {
    //     eprintln!(
    //         "Failed to fetch tile {}/{}/{}: {}",
    //         tile.z, tile.x, tile.y, e
    //     );
    //     opengl_helper::load_image("test.png") // your own function returning RgbaImage
    // });

    let mut viewport = Viewport {
        z: 1,
        center_x: 1.0,
        center_y: 1.0,
    };

    let mut tile_cache: LruCache<TilePos, gl::types::GLuint> =
        LruCache::new(NonZeroUsize::new(128).unwrap());
    let tile_cache_buf: Arc<Mutex<LruCache<TilePos, u8>>> =
        Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap())));

    let (job_tx, job_rx): (Sender<TilePos>, Receiver<TilePos>) = channel();
    let (res_tx, res_rx): (Sender<TileLoad>, Receiver<TileLoad>) = channel();
    let (server_tx, server_rx): (Sender<TilePos>, Receiver<TilePos>) = channel();
    let job_rx = Arc::new(Mutex::new(job_rx));

    for _ in 0..4 {
        let job_rx = job_rx.clone();
        let res_tx = res_tx.clone();
        //let tile_map = tile_map.clone();
        let tile_cache_buf = tile_cache_buf.clone();

        let server_tx = server_tx.clone();
        thread::spawn(move || {
            while let Ok(tile_pos) = job_rx.lock().unwrap().recv() {
                // perform blocking I/O off the main thread
                let tile_cache_result: Option<(TilePos, u8)> = {
                    let mut guard = tile_cache_buf.lock().unwrap();
                    guard
                        .get_key_value(&tile_pos)
                        // clone both key & value out of the map
                        .map(|(k, v)| (k.clone(), v.clone()))
                };
                match tile_cache_result {
                    None => {
                        {
                            {
                                tile_cache_buf.lock().unwrap().put(tile_pos, 0);
                            }
                            let tile_load = opengl_helper::fetch_tile(tile_pos);
                            match tile_load.unwrap() {
                                TileLoad::Loaded {
                                    texture,
                                    source_tile,
                                } => {
                                    tile_cache_buf.lock().unwrap().put(tile_pos, 2);
                                    let _ = res_tx.send(TileLoad::Loaded {
                                        texture,
                                        source_tile,
                                    });
                                }
                                TileLoad::Loading {
                                    texture,
                                    source_tile,
                                    target_tile,
                                } => {
                                    tile_cache_buf.lock().unwrap().put(tile_pos, 1);
                                    let _ = res_tx.send(TileLoad::Loading {
                                        texture,
                                        source_tile,
                                        target_tile,
                                    });
                                    //let _ = opengl_helper::fetch_tile_from_server(&tile_pos);
                                    let _ = server_tx.send(target_tile);
                                }
                                TileLoad::Failed {} => {
                                    let _ = opengl_helper::fetch_tile_from_server(&tile_pos);
                                }
                            }
                        }
                    }
                    Some(entry) => {
                        //let map_val = ;
                        if entry.1 == 3 {
                            let tile_load = opengl_helper::fetch_tile(tile_pos);
                            match tile_load.unwrap() {
                                TileLoad::Loaded {
                                    texture,
                                    source_tile,
                                } => {
                                    let _ = res_tx.send(TileLoad::Loaded {
                                        texture,
                                        source_tile,
                                    });
                                }
                                TileLoad::Loading {
                                    texture: _texture,
                                    source_tile: _source_tile,
                                    target_tile: _target_tile,
                                } => {}
                                TileLoad::Failed {} => {}
                            }
                        } else {
                        }
                    }
                }
            }
        });
    }

    {
        let res_tx = res_tx.clone();
        //let tile_cache_buf =  tile_cache_buf.clone();
        thread::spawn(move || {
            let mut buffer = VecDeque::new();

            loop {
                // Try to get as many messages as are pending
                match server_rx.try_recv() {
                    Ok(tile_pos) => {
                        buffer.push_back(tile_pos); // stack-like
                        if buffer.len() > 64 {
                            buffer.pop_front();
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        // Nothing new; process last-in item
                        if let Some(tile_pos) = buffer.pop_back() {
                            let tile_load = opengl_helper::fetch_tile_from_server(&tile_pos);
                            if let Ok(load) = tile_load {
                                let _ = res_tx.send(load);
                                println!(
                                    "Loaded Tile from web {}_{}_{}: {}",
                                    tile_pos.z, tile_pos.x, tile_pos.y, tile_pos.m
                                );
                            }
                        } else {
                            // Sleep briefly if there's no work to avoid busy spinning
                            thread::sleep(Duration::from_millis(12));
                        }
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        });
    }

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
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => viewport.pan(0.0, -0.25),
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => viewport.pan(0.0, 0.25),
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => viewport.pan(-0.25, 0.0),
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => viewport.pan(0.25, 0.0),
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => viewport.zoom_in(),
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    viewport.zoom_out();
                    //tile_map.clear();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    ..
                } => {
                    if map == 0 {
                        map = 1;
                    } else {
                        map = 0;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Kp0),
                    ..
                } => map = 0,
                Event::KeyDown {
                    keycode: Some(Keycode::Kp1),
                    ..
                } => map = 1,
                Event::KeyDown {
                    keycode: Some(Keycode::Kp2),
                    ..
                } => map = 2,
                Event::KeyDown {
                    keycode: Some(Keycode::Kp3),
                    ..
                } => map = 3,
                Event::KeyDown {
                    keycode: Some(Keycode::Kp4),
                    ..
                } => map = 4,
                Event::KeyDown {
                    keycode: Some(Keycode::Kp5),
                    ..
                } => map = 5,

                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    clicks: clicks_in_event,
                    x,
                    y,
                    ..
                } => {
                    let (w, h) = window.size();
                    if clicks_in_event >= 2 {
                        viewport.zoom_in_at_pixel(w, h, x, y);
                    } else {
                        // clicks == 1
                        viewport.center_on_pixel(w, h, x, y);
                    }
                }
                _ => {}
            }
        }

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        opengl_helper::draw_visible_tiles(
            &mut viewport,
            window.size().0,
            window.size().1,
            shader_program.0,
            vao.0,
            &mut tile_cache,
            map,
            job_tx.clone(),
        );
        window.gl_swap_window();
        while let Ok(tile_load) = res_rx.try_recv() {
            match tile_load {
                TileLoad::Loaded {
                    texture,
                    source_tile,
                } => {
                    let tex_id = opengl_helper::create_texture_from_bitmap(&texture);
                    tile_cache.put(source_tile, tex_id);
                }
                TileLoad::Loading {
                    texture,
                    source_tile: _source_tile,
                    target_tile,
                } => {
                    let tex_id = opengl_helper::create_texture_from_bitmap(&texture);
                    tile_cache.put(target_tile, tex_id);
                }
                TileLoad::Failed {} => {}
            }
        }
        ::std::thread::sleep(std::time::Duration::new(0, (1_000_000_000 / 60) as u32));
    }

    Ok(())
}
