extern crate gl;

use crate::opengl_helper;

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
