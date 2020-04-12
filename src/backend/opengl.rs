extern crate gl;
extern crate x11;

mod buffer;
pub mod setup;
mod shader;
mod texture;

use std::ffi::CString;
use std::ptr::null_mut;

use crate::window::Window;
use buffer::{Buffer, ElementBuffer, VertexArray};
use shader::{Program, Shader};
use texture::Texture;
use x11::{glx::*, xlib};

pub struct Opengl<'a> {
    pub ctx: *mut __GLXcontextRec,
    pub conn: &'a xcb::Connection,
    pub dpy: *mut xlib::Display,
    pub draw_win: xlib::XID,
    pub fbconfig: GLXFBConfig,
}

impl<'a> Opengl<'a> {
    pub fn init(
        conn: &xcb::Connection,
        screens: i32,
        win: xcb::Window,
    ) -> Result<Opengl, &str> {
        setup::verify_extensions(conn, screens)?;
        // setup framebuffer context
        let fbc = setup::get_glxfbconfig(
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

        conn.flush();
        let glx_bind_tex_image: setup::GLXBindTexImageEXT = unsafe {
            std::mem::transmute(setup::load_gl_func("glXBindTexImageEXT"))
        };
        let glx_release_tex_image: setup::GLXReleaseTexImageEXT = unsafe {
            std::mem::transmute(setup::load_gl_func("glXReleaseTexImageEXT"))
        };

        // load all function pointers
        unsafe { gl::load_with(|n| setup::load_gl_func(&n)) };

        if !gl::GenVertexArrays::is_loaded() {
            return Err("no GL3 support available!");
        }
        let ctx = setup::create_glx_context(conn, fbc)?;

        unsafe {
            glXMakeCurrent(conn.get_raw_dpy(), win as xlib::XID, ctx);
        }
        conn.flush();
        ////////////
        ////////////
        let vert_shader = Shader::from_vert_source(
            &CString::new(include_str!("opengl/triangle.vert")).unwrap(),
        )
        .unwrap();

        let frag_shader = Shader::from_frag_source(
            &CString::new(include_str!("opengl/triangle.frag")).unwrap(),
        )
        .unwrap();

        let shader_program =
            Program::from_shaders(&[vert_shader, frag_shader]).unwrap();

        let vertices: Vec<f32> = vec![
            // positions     // colors
            -0.5, -0.5, 0.0, 1.0, 0.0, 0.0, // bottom right
            0.5, -0.5, 0.0, 0.0, 1.0, 0.0, // bottom left
            0.0, 0.5, 0.0, 0.0, 0.0, 1.0, // top
        ];

        let vbo = Buffer::new();
        vbo.bind();
        vbo.load_data(&vertices);

        let vao = VertexArray::new();
        vao.bind();
        vbo.bind();
        VertexArray::enable(0);
        VertexArray::attrib_pointer(0, 3, 6, 0);
        VertexArray::enable(1);
        VertexArray::attrib_pointer(1, 3, 6, 3);
        Buffer::unbind();
        VertexArray::unbind();

        //////////// draw
        unsafe {
            gl::ClearColor(0.3, 0.3, 0.4, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            shader_program.set_used();

            vao.bind();
            gl::DrawArrays(gl::TRIANGLES, 0, 3);

            setup::check_gl_error();
            glXSwapBuffers(conn.get_raw_dpy(), win as xlib::XID);
        }

        ////////////
        ////////////
        Ok(Opengl {
            ctx,
            conn,
            dpy: conn.get_raw_dpy(),
            draw_win: win as xlib::XID,
            fbconfig: fbc,
        })
    }

    pub fn draw_window(&self, window: &Window) {
        let vert = Shader::from_vert_source(
            &CString::new(include_str!("opengl/window.vert")).unwrap(),
        )
        .unwrap();
        let frag = Shader::from_frag_source(
            &CString::new(include_str!("opengl/window.frag")).unwrap(),
        )
        .unwrap();
        let prog = Program::from_shaders(&[vert, frag]).unwrap();
        prog.set_used();

        let vao = VertexArray::new();
        vao.bind();

        let vbo = Buffer::new();
        vbo.bind();
        vbo.load_data(&[
            // top left
            (window.x as f32) / 1366.0 * 2.0 - 1.0,
            (window.y as f32) / 768.0 * -2.0 + 1.0,
            0.0, 0.0,
            // top right
            (window.x as f32 + window.width as f32) / 1366.0 * 2.0 - 1.0,
            (window.y as f32) / 768.0 * -2.0 + 1.0,
            1.0, 0.0,
            // bottom left
            (window.x as f32) / 1366.0 * 2.0 - 1.0,
            (window.y as f32 + window.height as f32) / 768.0 * -2.0 + 1.0,
            0.0, 1.0,
            // bottom right
            (window.x as f32 + window.width as f32) / 1366.0 * 2.0 - 1.0,
            (window.y as f32 + window.height as f32) / 768.0 * -2.0 + 1.0,
            1.0, 1.0,
        ]);

        VertexArray::enable(0);
        VertexArray::attrib_pointer(0, 2, 4, 0);
        VertexArray::enable(1);
        VertexArray::attrib_pointer(1, 2, 4, 2);

        let ebo = ElementBuffer::new();
        ebo.bind();
        ebo.load_data(&[0, 1, 2, 1, 2, 3]);

        let texture =
            Texture::from_pixmap(window.pixmap, self.dpy, self.fbconfig);
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

            gl::ClearColor(0.4, 0.4, 0.5, 1.0);
            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            gl::Flush();
            setup::check_gl_error();
            glXSwapBuffers(self.dpy, self.draw_win);
        }
        self.conn.flush();
    }
    pub fn clear(&self) {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }
}

impl<'a> Drop for Opengl<'a> {
    fn drop(&mut self) {
        unsafe { glXDestroyContext(self.dpy, self.ctx) };
        self.conn.flush();
    }
}
