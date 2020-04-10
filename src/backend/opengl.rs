extern crate gl;
extern crate x11;

mod buffer;
mod opengl;
mod shader;

use std::ffi::CString;
use std::ptr::null_mut;

use crate::window::Window;
use buffer::{Buffer, VertexArray};
use shader::{Program, Shader};
use x11::{glx::*, xlib};

pub struct Opengl<'a> {
    pub ctx: *mut __GLXcontextRec,
    pub conn: &'a xcb::Connection,
    pub dpy: *mut xlib::Display,
    pub draw_win: xlib::XID,
}

impl<'a> Opengl<'a> {
    pub fn init(
        conn: &xcb::Connection,
        screens: i32,
        win: xcb::Window,
    ) -> Result<Opengl, &str> {
        opengl::verify_extensions(conn, screens)?;
        // setup framebuffer context
        let fbc = opengl::get_glxfbconfig(
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
        let _glx_bind_tex_image: opengl::GLXBindTexImageEXT = unsafe {
            std::mem::transmute(opengl::load_gl_func("glXBindTexImageEXT"))
        };
        let _glx_release_tex_image: opengl::GLXReleaseTexImageEXT = unsafe {
            std::mem::transmute(opengl::load_gl_func("glXReleaseTexImageEXT"))
        };

        // load all function pointers
        unsafe { gl::load_with(|n| opengl::load_gl_func(&n)) };

        if !gl::GenVertexArrays::is_loaded() {
            return Err("no GL3 support available!");
        }
        let ctx = opengl::create_glx_context(conn, fbc)?;

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

            opengl::check_gl_error();
            glXSwapBuffers(conn.get_raw_dpy(), win as xlib::XID);
        }

        ////////////
        ////////////
        Ok(Opengl {
            ctx,
            conn,
            dpy: conn.get_raw_dpy(),
            draw_win: win as xlib::XID,
        })
    }

    pub fn draw_window(&self, window: &Window) {
        unsafe {
            glXMakeCurrent(self.dpy, self.draw_win, self.ctx);
            gl::ClearColor(0.3f32, 0.3f32, 0.4f32, 1.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Flush();
            opengl::check_gl_error();
            glXSwapBuffers(self.dpy, self.draw_win);
            glXMakeCurrent(self.dpy, 0, null_mut());
        }
        self.conn.flush();
    }
}

impl<'a> Drop for Opengl<'a> {
    fn drop(&mut self) {
        unsafe { glXDestroyContext(self.dpy, self.ctx) };
        self.conn.flush();
    }
}
