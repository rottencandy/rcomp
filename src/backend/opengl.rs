extern crate gl;
extern crate x11;

mod buffer;
pub mod setup;
mod shader;
pub mod texture;

use std::ffi::CString;

use crate::window::Window;
use buffer::{Buffer, ElementBuffer, VertexArray};
use shader::{Program, Shader};
use texture::Texture;
use x11::{glx::*, xlib};

// texture_from_pixmap extension constants
const GLX_TEXTURE_TARGET_EXT: i32 = 0x20D6;
const GLX_TEXTURE_2D_EXT: i32 = 0x20DC;
const GLX_TEXTURE_FORMAT_EXT: i32 = 0x20D5;
const GLX_TEXTURE_FORMAT_RGBA_EXT: i32 = 0x20DA;
const GLXFRONT_LEFT_EXT: i32 = 0x20DE;

pub struct Opengl<'a> {
    pub ctx: *mut __GLXcontextRec,
    pub conn: &'a xcb::Connection,
    pub dpy: *mut xlib::Display,
    pub draw_win: xlib::XID,
    pub fbconfig: GLXFBConfig,

    // I don't yet know enough Rust to do this any other way
    glx_bind_tex_image: setup::GLXBindTexImageEXT,
    glx_release_tex_image: setup::GLXReleaseTexImageEXT,

    vbo: Buffer,
    vao: VertexArray,
    ebo: ElementBuffer,
}

impl<'a> Opengl<'a> {
    pub fn init(
        conn: &xcb::Connection,
        screens: i32,
        win: xcb::Window,
    ) -> Result<Opengl, &str> {
        setup::verify_extensions(conn, screens)?;
        // setup framebuffer context
        let fbconfig = setup::get_glxfbconfig(
            conn.get_raw_dpy(),
            screens,
            &[
                GLX_X_RENDERABLE,
                1,
                GLX_DRAWABLE_TYPE,
                GLX_WINDOW_BIT,
                GLX_RENDER_TYPE,
                GLX_RGBA_BIT,
                GLX_X_VISUAL_TYPE,
                GLX_TRUE_COLOR,
                GLX_RED_SIZE,
                8,
                GLX_GREEN_SIZE,
                8,
                GLX_BLUE_SIZE,
                8,
                GLX_ALPHA_SIZE,
                8,
                GLX_DEPTH_SIZE,
                24,
                GLX_STENCIL_SIZE,
                8,
                GLX_DOUBLEBUFFER,
                1,
                0,
            ],
        )?;

        // load all function pointers
        unsafe { gl::load_with(|n| setup::load_gl_func(&n)) };
        // load extension functions
        let glx_bind_tex_image: setup::GLXBindTexImageEXT = unsafe {
            std::mem::transmute(setup::load_gl_func("glXBindTexImageEXT"))
        };
        let glx_release_tex_image: setup::GLXReleaseTexImageEXT = unsafe {
            std::mem::transmute(setup::load_gl_func("glXReleaseTexImageEXT"))
        };

        if !gl::GenVertexArrays::is_loaded() {
            return Err("no GL3 support available!");
        }
        let ctx = setup::create_glx_context(conn, fbconfig)?;

        unsafe {
            // Set ctx as the current one used for drawing
            glXMakeCurrent(conn.get_raw_dpy(), win as xlib::XID, ctx);
            // Use pixmap texture's alpha to calculate transparency
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
        }

        // Load shaders
        let vert = Shader::from_vert_source(
            &CString::new(include_str!("opengl/window.vert")).unwrap(),
        )
        .unwrap();
        let frag = Shader::from_frag_source(
            &CString::new(include_str!("opengl/window.frag")).unwrap(),
        )
        .unwrap();
        let shader_program = Program::from_shaders(&[vert, frag]).unwrap();
        // Provide screen dimensions
        let screen_dim = shader_program.create_uniform("screenDim");
        shader_program.set_used();
        screen_dim.data_2f(&[1366.0, 768.0]);

        // Vertex object and array
        let vao = VertexArray::new();
        vao.bind();
        let vbo = Buffer::new();
        vbo.bind();
        // for pos co-ordinates
        VertexArray::enable(0);
        VertexArray::attrib_pointer(0, 2, 4, 0);
        // for texture co-ordinates
        VertexArray::enable(1);
        VertexArray::attrib_pointer(1, 2, 4, 2);

        // Element buffer array
        let ebo = ElementBuffer::new();
        ebo.bind();
        ebo.load_data(&[0, 1, 2, 1, 2, 3]);

        Ok(Opengl {
            ctx,
            conn,
            dpy: conn.get_raw_dpy(),
            draw_win: win as xlib::XID,
            fbconfig,
            glx_bind_tex_image,
            glx_release_tex_image,

            // Even if we don't directly use these, they have to
            // remain in scope so their context doesn't get deleted
            vbo,
            vao,
            ebo,
        })
    }

    pub fn draw_window(&self, window: &Window) {
        // TODO: Look into geometry shader
        self.vbo.load_data(&[
            // top left
            (window.x as f32),
            (window.y as f32),
            0.0,
            0.0,
            // top right
            (window.x as f32
                + window.width as f32
                + window.border_width as f32 * 2.0),
            (window.y as f32),
            1.0,
            0.0,
            // bottom left
            (window.x as f32),
            (window.y as f32
                + window.height as f32
                + window.border_width as f32 * 2.0),
            0.0,
            1.0,
            // bottom right
            (window.x as f32
                + window.width as f32
                + window.border_width as f32 * 2.0),
            (window.y as f32
                + window.height as f32
                + window.border_width as f32 * 2.0),
            1.0,
            1.0,
        ]);

        window.texture.bind();
        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            setup::check_gl_error();
        }
    }

    pub fn update_glxpixmap(&self, win: &mut Window) {
        win.update_pixmap(self.conn);
        win.glxpixmap = unsafe {
            setup::glXCreatePixmap(
                self.dpy,
                self.fbconfig,
                win.pixmap as u64,
                [
                    GLX_TEXTURE_TARGET_EXT,
                    GLX_TEXTURE_2D_EXT,
                    GLX_TEXTURE_FORMAT_EXT,
                    GLX_TEXTURE_FORMAT_RGBA_EXT,
                    xcb::NONE as i32,
                ]
                .as_ptr(),
            )
        };
    }

    pub fn update_window_texture(&self, win: &mut Window) {
        win.texture = Texture::new();
        win.texture.bind();
        unsafe {
            (self.glx_bind_tex_image)(
                self.dpy,
                win.glxpixmap,
                0x20de,
                std::ptr::null(),
            );
        }
        // Set texture parameters
        unsafe {
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MIN_FILTER,
                gl::NEAREST as i32,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_MAG_FILTER,
                gl::NEAREST as i32,
            );
        }
    }

    pub fn render(&self) {
        unsafe {
            glXSwapBuffers(self.dpy, self.draw_win);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            //gl::ClearColor(0.4, 0.4, 0.5, 1.0);
        }
    }
}

impl<'a> Drop for Opengl<'a> {
    fn drop(&mut self) {
        unsafe { glXDestroyContext(self.dpy, self.ctx) };
        self.conn.flush();
    }
}
