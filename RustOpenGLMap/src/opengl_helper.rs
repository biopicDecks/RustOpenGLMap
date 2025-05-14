extern crate gl;

use crate::opengl_helper;
use crate::tile::TileLoad;
use crate::tile::TilePos;
use crate::viewport::Viewport;
use curl::easy::Easy;
use gl::types::*;
use image::ImageReader;
use image::RgbaImage;
use lru::LruCache;
use std::error::Error;
// curl = "0.4"
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use once_cell::sync::Lazy;
use tokio;

macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const gl::types::GLchar
    };
}

pub static USER_AGENT: Lazy<String> = Lazy::new(|| {
    format!(
        "{}/{} (+{}; {})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        "https://github.com/biopicDecks/RustOpenGLMap",
        "biopic.decks-0w@icloud.com"
    )
});

// Define the result type that worker threads will send back
#[derive(Debug)] // For easier debugging
pub enum TileLoadResult {
    // Original requested TilePos, Image data, Actual TilePos of the loaded image (if fallback)
    Success(TilePos, RgbaImage, TilePos),
    // Original requested TilePos, Error message
    Failure(TilePos, String),
}

/// Represents a vertex array object (VAO) in OpenGL.
///
/// A VAO stores the state of vertex attribute pointers, allowing for efficient rendering
/// of geometry without repeatedly specifying the layout of vertex data.
///
/// # Fields
///
/// * `0`: The OpenGL identifier for the vertex array object.  This is an unsigned integer
///   returned by `gl::GenVertexArrays`.
///
/// # Example
///
/// ```
/// use gl;
///
/// // Assuming a VAO has been created and its identifier is stored in `vao_id`
/// let vao = VertexArray(vao_id);
/// ```
pub struct VertexArray(pub gl::types::GLuint);
impl VertexArray {
    /// Creates a new Vertex Array Object (VAO).
    ///
    /// This function generates a new VAO using `gl::GenVertexArrays`.
    /// It returns `Some(VAO)` if the VAO is successfully created,
    /// and `None` if the creation fails.
    ///
    /// # Returns
    ///
    /// * `Some(Self)` - A `VAO` struct containing the newly generated VAO ID.
    /// * `None` - If the VAO creation fails (e.g., due to resource exhaustion).
    ///
    /// # Panics
    ///
    /// This function does not panic. It returns `None` in case of failure.
    pub fn new() -> Option<Self> {
        let mut vao = 0;
        unsafe { gl::GenVertexArrays(1, &mut vao) };
        if vao != 0 { Some(Self(vao)) } else { None }
        //assert_ne!(vao, 0);
    }
    /// Binds the vertex array object.
    ///
    /// This function binds the vertex array object stored within the struct to the current rendering context.
    /// It uses the `gl` crate's `BindVertexArray` function to perform the binding.
    ///
    /// # Safety
    ///
    /// This function uses `unsafe` code because it directly calls OpenGL functions.  Ensure that the vertex array object is valid and has been created before calling this function.  Incorrect usage can lead to crashes or undefined behavior.
    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.0);
        }
    }
    /// Unbinds the currently bound vertex array object (VAO).
    ///
    /// This function releases the association between the current rendering context and the currently bound VAO.
    /// It effectively resets the VAO to an unbound state.
    ///
    /// # Safety
    ///
    /// This function uses `gl::BindVertexArray` which is an unsafe OpenGL function.  Ensure that a valid OpenGL context is active before calling this function.
    pub fn clear_binding() {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}

/// The types of buffer object that you can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// Array Buffers holds arrays of vertex data for drawing.
    Array = gl::ARRAY_BUFFER as isize,
    /// Element Array Buffers hold indexes of what vertexes to use for drawing.
    ElementArray = gl::ELEMENT_ARRAY_BUFFER as isize,
}
pub struct Buffer(pub gl::types::GLuint);
impl Buffer {
    pub fn new() -> Option<Self> {
        let mut vbo = 0;
        unsafe { gl::GenBuffers(1, &mut vbo) };
        if vbo != 0 { Some(Self(vbo)) } else { None }
        //assert_ne!(vao, 0);
    }
    pub fn bind(&self, ty: BufferType) {
        unsafe {
            gl::BindBuffer(ty as gl::types::GLenum, self.0);
        }
    }
    pub fn clear_binding(ty: BufferType) {
        unsafe {
            gl::BindBuffer(ty as gl::types::GLenum, 0);
        }
    }

    /// Places a slice of data into a previously-bound buffer.
    pub fn data(ty: BufferType, data: &[u8], usage: gl::types::GLenum) {
        unsafe {
            gl::BufferData(
                ty as gl::types::GLenum,
                data.len().try_into().unwrap(),
                data.as_ptr().cast(),
                usage,
            );
        }
    }
}

/// The types of shader object.
pub enum ShaderType {
    /// Vertex shaders determine the position of geometry within the screen.
    Vertex = gl::VERTEX_SHADER as isize,
    /// Fragment shaders determine the color output of geometry.
    ///
    /// Also other values, but mostly color.
    Fragment = gl::FRAGMENT_SHADER as isize,
}

pub struct Shader;
impl Shader {
    pub fn compile_shader(shader_type: ShaderType, shader_code: &str) -> gl::types::GLenum {
        unsafe {
            let shader = gl::CreateShader(shader_type as gl::types::GLenum);
            assert_ne!(shader, 0);
            gl::ShaderSource(
                shader,
                1,
                &(shader_code.as_bytes().as_ptr().cast()),
                &(shader_code.len().try_into().unwrap()),
            );
            gl::CompileShader(shader);

            let mut success = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut v: Vec<u8> = Vec::with_capacity(1024);
                let mut log_len = 0_i32;
                gl::GetShaderInfoLog(shader, 1024, &mut log_len, v.as_mut_ptr().cast());
                v.set_len(log_len.try_into().unwrap());
                panic!("Compile Error: {}", String::from_utf8_lossy(&v));
            } else {
                println!("Shader Compiled Succccesfully");
            }
            shader
        }
    }
}

pub struct ShaderProgram(pub gl::types::GLuint);
impl ShaderProgram {
    pub fn new() -> Option<Self> {
        let prog = unsafe { gl::CreateProgram() };
        if prog != 0 { Some(Self(prog)) } else { None }
    }
    pub fn from_vert_frag(vert_str: &str, frag_str: &str) -> Result<Self, String> {
        // Vertex Shader
        let shader_program =
            Self::new().ok_or_else(|| "Couldn't allocate a program".to_string())?;
        let vertex_shader =
            opengl_helper::Shader::compile_shader(opengl_helper::ShaderType::Vertex, vert_str);
        let frag_shader =
            opengl_helper::Shader::compile_shader(opengl_helper::ShaderType::Fragment, frag_str);

        unsafe {
            gl::AttachShader(shader_program.0, vertex_shader);
            gl::AttachShader(shader_program.0, frag_shader);
            gl::LinkProgram(shader_program.0);
            let mut success = 0;
            gl::GetProgramiv(shader_program.0, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut v: Vec<u8> = Vec::with_capacity(1024);
                let mut log_len = 0_i32;
                gl::GetProgramInfoLog(shader_program.0, 1024, &mut log_len, v.as_mut_ptr().cast());
                v.set_len(log_len.try_into().unwrap());
                let out = format!("Program Link Error: {}", String::from_utf8_lossy(&v));
                shader_program.delete();
                Err(out)
            } else {
                println!("Shader's Linked Successfully");
                // clean up
                gl::DeleteShader(vertex_shader);
                gl::DeleteShader(frag_shader);

                // set program
                gl::UseProgram(shader_program.0);
                Ok(shader_program)
            }
        }
    }

    pub fn delete(self) {
        unsafe { gl::DeleteProgram(self.0) };
    }
}
pub fn load_image(path: &str) -> image::RgbaImage {
    let img = ImageReader::open(path)
        .expect("Failed to open image")
        .decode()
        .expect("Failed to decode image");
    let mut rgba_image = img.to_rgba8();
    // Flip scanlines (vertical flip) if needed
    image::imageops::flip_vertical_in_place(&mut rgba_image);
    rgba_image
}
pub fn fetch_tile_from_server(tile: &TilePos) -> Result<TileLoad, Box<dyn Error>> {
    // Pre‑allocate ~8KiB to avoid repeated reallocations for small tiles.
    let mut data: Vec<u8> = Vec::with_capacity(8 * 1024);

    // --- libcurl setup -----------------------------------------------------
    let mut easy = Easy::new();

    let mut content_type = String::new();
    let mut response_code = 0;

    let mut count = 0;

    while response_code != 200 && (tile.m == 1 && count == 0) || (tile.m == 0 && count == 0) {
        let url;
        if tile.m == 0 {
            url = format!(
                "https://tile.openstreetmap.org/{}/{}/{}.png",
                tile.z, tile.x, tile.y
            );
        } else {
            url = format!(
                "https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{}/{}/{}",
                tile.z, tile.y, tile.x
            );
        }
        easy.url(&url)?;
        easy.follow_location(true)?;
        easy.useragent(&USER_AGENT)?; // <- sets the HTTP User‑Agent header
        // --- Perform the HTTP GET ---------------------------------------------
        {
            let mut transfer = easy.transfer();

            transfer.header_function(|header| {
                let header_str = String::from_utf8_lossy(header);
                if header_str.to_ascii_lowercase().starts_with("content-type:") {
                    content_type = header_str["content-type:".len()..].trim().to_string();
                }
                true
            })?;

            transfer.write_function(|chunk| {
                data.write_all(chunk).unwrap();
                Ok(chunk.len())
            })?;
            transfer.perform()?; // propagate any HTTP/network error
        }
        response_code = easy.response_code().unwrap_or(0);

        if response_code != 200 {
            return Err(Box::from(format!("HTTP error: {}", response_code)));
        }

        if data.len() < 4 {
            return Err(Box::from(
                "Downloaded data too small to be valid image".to_string(),
            ));
        }

        // must be a png or jpg
        if &data[0..4] != b"\x89PNG" && &data[0..2] != b"\xFF\xD8" {
            return Err(Box::from("Not a PNG or JPEG".to_string()));
        }

        count = count + 1;
    }
    // --- Decode PNG into RGBA8 --------------------------------------------
    let img = image::load_from_memory(&data)?;
    let mut img_rgba = img.to_rgba8();
    let disk: PathBuf;
    if tile.m == 0 {
        disk = format!("Tiles/OSMTile_{}_{}_{}.png", tile.z, tile.x, tile.y).into();
    } else {
        disk = format!("Tiles/ESRITile_{}_{}_{}.png", tile.z, tile.x, tile.y).into();
    }
    img_rgba.save(disk)?;
    image::imageops::flip_vertical_in_place(&mut img_rgba); // GL wants origin‑bottom‑left
    let tile_state = TileLoad::Loaded {
        texture: img_rgba,
        source_tile: *tile,
    };
    Ok(tile_state)
}
pub fn get_file_path(loaded_tile: TilePos) -> PathBuf {
    if loaded_tile.m == 0 {
        format!(
            "Tiles/OSMTile_{}_{}_{}.png",
            loaded_tile.z, loaded_tile.x, loaded_tile.y
        )
        .into()
    } else {
        format!(
            "Tiles/ESRITile_{}_{}_{}.png",
            loaded_tile.z, loaded_tile.x, loaded_tile.y
        )
        .into()
    }
}

pub fn fetch_tile(tile: TilePos) -> Result<TileLoad, Box<dyn Error>> {
    let mut disk: PathBuf;
    let mut tile_state = TileLoad::Failed;
    let mut first_load = true;
    let mut loaded_tile = tile;
    while tile_state == TileLoad::Failed && (first_load || tile.z > 0) {
        disk = get_file_path(loaded_tile);
        // if loaded_tile.m == 0
        // {
        //     disk = format!("Tiles/OSMTile_{}_{}_{}.png", loaded_tile.z, loaded_tile.x, loaded_tile.y).into();
        // } else {
        //     disk = format!("Tiles/ESRITile_{}_{}_{}.png", loaded_tile.z, loaded_tile.x, loaded_tile.y).into();
        // }
        if disk.exists() {
            println!("load from disk");
            let image_open = ImageReader::open(&disk)
                .unwrap()
                .with_guessed_format()
                .unwrap() // detect by magic bytes
                .decode(); // dynamic image
            match image_open {
                Ok(img) => {
                    if first_load {
                        let mut img_rgba = img.to_rgba8(); // hard‑convert to RGBA8
                        image::imageops::flip_vertical_in_place(&mut img_rgba); // GL wants origin‑bottom‑left
                        //let id = create_texture_from_bitmap(&img_rgba);
                        tile_state = TileLoad::Loaded {
                            texture: img_rgba,
                            source_tile: loaded_tile,
                        };
                    } else {
                        let (x, y, width, height) = loaded_tile.get_crop(&tile);
                        let cropped = img.crop_imm(x as u32, y as u32, width as u32, height as u32);
                        //let resized = cropped.resize_exact(256, 256, image::imageops::FilterType::Nearest);
                        let mut img_rgba = cropped.to_rgba8(); // hard‑convert to RGBA8
                        image::imageops::flip_vertical_in_place(&mut img_rgba); // GL wants origin‑bottom‑left
                        //let id = create_texture_from_bitmap(&img_rgba);
                        tile_state = TileLoad::Loading {
                            texture: img_rgba,
                            source_tile: loaded_tile,
                            target_tile: tile,
                        };
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to open tile, loading from web {}_{}_{}: {}",
                        loaded_tile.z, loaded_tile.x, loaded_tile.y, e
                    );
                    delete_file(disk);
                }
            }
        }
        first_load = false;
        loaded_tile.zoom_out();
    }
    Ok(tile_state)
}

pub fn create_texture_from_bitmap(bitmap: &RgbaImage) -> GLuint {
    let mut texture: GLuint = 0;

    let (width, height) = (bitmap.width(), bitmap.height());
    let pixels = bitmap.as_raw(); // Gets &[u8] of pixel data

    unsafe {
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1); // <- makes any width safe

        // Texture wrapping
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);

        // Texture filtering
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);

        // Upload texture data
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,                 // mipmap level
            gl::RGBA as GLint, // internal format
            width as GLsizei,
            height as GLsizei,
            0,                 // border
            gl::RGBA,          // input format
            gl::UNSIGNED_BYTE, // input type
            pixels.as_ptr() as *const GLvoid,
        );

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::GenerateMipmap(gl::TEXTURE_2D);
    }
    texture
}

/// The polygon display modes you can set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    /// Just show the points.
    Point = gl::POINT as isize,
    /// Just show the lines.
    Line = gl::LINE as isize,
    /// Fill in the polygons.
    Fill = gl::FILL as isize,
}

/// Sets the font and back polygon mode to the mode given.
pub fn polygon_mode(mode: PolygonMode) {
    unsafe { gl::PolygonMode(gl::FRONT_AND_BACK, mode as GLenum) };
}

// pub fn create_texture(tile: TilePos, map: u8) -> TileState {
//     let bitmap = opengl_helper::fetch_tile(tile).unwrap_or_else(|e| {
//         eprintln!(
//             "Failed to fetch tile {}/{}/{}: {}",
//             tile.z, tile.x, tile.y, e
//         );
//         (load_image("test.png"),TilePos::new()) // your own function returning RgbaImage
//     });
//     create_texture_from_bitmap(&bitmap.0)
// }

pub fn draw_visible_tiles(
    vp: &mut Viewport,
    win_w: u32,
    win_h: u32,
    shader: u32, // program id
    vao: u32,
    tile_cache: &mut LruCache<TilePos, gl::types::GLuint>,
    map: u8,
    job_tx: Sender<TilePos>,
) {
    unsafe {
        gl::UseProgram(shader);
    }

    // tile-size expressed in Normalised Device Coordinates
    let scale_x = (256.0 / win_w as f64) * 2.0;
    let scale_y = (256.0 / win_h as f64) * 2.0;

    let scale_loc = unsafe { gl::GetUniformLocation(shader, c_str!("u_scale")) };
    let offset_loc = unsafe { gl::GetUniformLocation(shader, c_str!("u_offset")) };
    let texture_loc = unsafe { gl::GetUniformLocation(shader, c_str!("the_texture")) }; // Get location

    unsafe {
        gl::Uniform2f(scale_loc, scale_x as f32, scale_y as f32);
        gl::Uniform1i(texture_loc, 0); // Tell "the_texture" to use texture unit 0
    }

    // how many tiles we need around the centre
    let tiles_x = (win_w as f64 / 256.0).ceil() as i32 + 2;
    let tiles_y = (win_h as f64 / 256.0).ceil() as i32 + 2;

    unsafe {
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindVertexArray(vao);
    }
    let z_max = (1 << vp.z) - 1;
    let m_y = vp.center_y.floor() - tiles_y as f64 / 2.0;
    let ma_y = vp.center_y.ceil() + tiles_y as f64 / 2.0;
    let m_x = vp.center_x.floor() - tiles_x as f64 / 2.0;
    let ma_x = vp.center_x.ceil() + tiles_x as f64 / 2.0;
    for ty in m_y as i32..=ma_y as i32 {
        for tx in m_x as i32..=ma_x as i32 {
            if tx < 0 || ty < 0 {
                continue;
            }
            if tx > z_max || ty > z_max {
                continue;
            }

            let pos = TilePos {
                z: vp.z,
                x: tx as u32,
                y: ty as u32,
                m: map,
            };
            // get or download the texture for this tile -------------
            let state = tile_cache.get_key_value(&pos);
            match state {
                Some(tile_state) => {
                    let dx = tx as f64 - vp.center_x;
                    let dy = ty as f64 - vp.center_y;
                    // set per-tile translation in NDC -----------------------
                    let ofs_x = (dx) * scale_x;
                    let ofs_y = -(dy) * scale_y; // window Y is flipped
                    unsafe {
                        gl::Uniform2f(offset_loc, ofs_x as f32, ofs_y as f32);
                        gl::BindTexture(gl::TEXTURE_2D, *tile_state.1);
                        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
                    }
                    // if *tile_state.0 != pos
                    // {
                    //     let _ = job_tx.send(pos);
                    //     tile_cache.pop(&pos);
                    // }
                }
                None => {
                    let _ = job_tx.send(pos);
                }
            }

            // match tile_state {
            //     TileState::Loaded{texture_id, source_tile} => {
            //         unsafe {
            //             gl::BindTexture(gl::TEXTURE_2D, texture_id);
            //             gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            //         }
            //     },
            //     TileState::Loading{texture_id, source_tile} => {
            //         unsafe {
            //             gl::BindTexture(gl::TEXTURE_2D, texture_id);
            //             gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            //         }
            //     },
            //     TileState::Failed{} => {
            //     }
            // }
        }
    }
}
// This new function initiates an asynchronous tile load.
// It's called by draw_visible_tiles.
// pub async fn request_tile_load_async(
//     tile_to_load: TilePos,
//     result_sender: Sender<TileLoadResult>,
// ) {
//     thread::spawn(move || {
//         // The try_fetch_recursive function will attempt to load the tile,
//         // and fall back to parent tiles if necessary.
//         // It needs the original Z level to know when to stop falling back, or use a max depth.
//         match fetch_tile(tile_to_load) {
//             Ok((image_data, actual_loaded_pos)) => {
//                 if let Err(e) = result_sender.send(TileLoadResult::Success(tile_to_load, image_data, actual_loaded_pos)) {
//                     eprintln!("Failed to send loaded tile data to main thread: {}", e);
//                 }
//             }
//             Err(err_msg) => {
//                 if let Err(e) = result_sender.send(TileLoadResult::Failure(tile_to_load, err_msg)) {
//                     eprintln!("Failed to send tile load failure to main thread: {}", e);
//                 }
//             }
//         }
//     });
// }

pub async fn delete_file_async(path: &str) -> Result<(), std::io::Error> {
    tokio::fs::remove_file(path).await
}

#[tokio::main]
async fn delete_file(s: PathBuf) {
    // Delete file.txt asynchronously
    delete_file_async(s.to_str().unwrap()).await.unwrap();
}
