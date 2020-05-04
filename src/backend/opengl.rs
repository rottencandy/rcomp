extern crate gl;
extern crate x11;

pub mod buffer;
pub mod setup;
mod shader;
pub mod texture;

use crate::state::State;
use std::ffi::CString;
use std::os::raw::c_ulong;

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
const GLX_FRONT_LEFT_EXT: i32 = 0x20DE;

pub struct Opengl<'a> {
    pub ctx: *mut __GLXcontextRec,
    pub conn: &'a xcb::Connection,
    pub dpy: *mut xlib::Display,
    pub target_win: xlib::XID,
    pub fbconfig: GLXFBConfig,
    pub root_data_vbo: Buffer,
    pub root_texture: Texture,

    // I don't yet know enough Rust to do this any other way
    glx_bind_tex_image: setup::GLXBindTexImageEXT,
    glx_release_tex_image: setup::GLXReleaseTexImageEXT,

    _vao: VertexArray,
    _ebo: ElementBuffer,
}

impl<'a> Opengl<'a> {
    pub fn init(state: &State) -> Result<Opengl, &str> {
        setup::verify_extensions(&state.conn, state.xlib_screens)?;
        let raw_dpy = state.conn.get_raw_dpy();
        // setup framebuffer context
        let fbconfig = setup::get_glxfbconfig(
            raw_dpy,
            state.xlib_screens,
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
        let ctx = setup::create_glx_context(&state.conn, fbconfig)?;

        unsafe {
            // Set ctx as the current one used for drawing
            glXMakeCurrent(raw_dpy, state.overlay as xlib::XID, ctx);
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
        // TODO: Provide screen dimensions
        let screen_dim = shader_program.create_uniform("screenDim");
        shader_program.set_used();
        screen_dim.data_2f(&[1366.0, 768.0]);

        // Vertex object and array
        let vao = VertexArray::new();
        vao.bind();
        // for pos co-ordinates
        VertexArray::enable(0);
        // for texture co-ordinates
        VertexArray::enable(1);

        // Element buffer array
        let ebo = ElementBuffer::new();
        ebo.bind();
        ebo.load_data(&[0, 1, 2, 1, 2, 3]);

        // create root texture from pixmap
        let root_texture = {
            let root_glxpixmap = unsafe {
                setup::glXCreatePixmap(
                    raw_dpy,
                    fbconfig,
                    state.root.pixmap as u64,
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
            let texture = Texture::new();
            texture.bind();
            unsafe {
                (glx_bind_tex_image)(
                    raw_dpy,
                    root_glxpixmap,
                    GLX_FRONT_LEFT_EXT,
                    std::ptr::null(),
                );
            }
            set_tex_params();
            texture
        };

        // Root dimensions data
        let root_data_vbo = {
            let vbo = Buffer::new();
            vbo.bind();
            vbo.load_data(&formatted_win_data(&state.root));
            vbo
        };

        Ok(Opengl {
            ctx,
            conn: &state.conn,
            dpy: raw_dpy,
            target_win: state.overlay as xlib::XID,
            fbconfig,
            root_texture,
            root_data_vbo,
            glx_bind_tex_image,
            glx_release_tex_image,

            // Even if we don't directly use these, they have to
            // remain in scope so their context doesn't get deleted
            _vao: vao,
            _ebo: ebo,
        })
    }

    pub fn init_window(&self, win: &mut Window) {
        // No point in creating texture or glxpixmap since they have
        // to be re created for every update
        win.context.vbo = Buffer::new();
        win.context.load_buffer(win);
    }

    pub fn update_pixmap(&self, win: &mut Window) {
        // The texture is only updated on `update_texture`
        // so no need to bind yet
        win.context.texture = Texture::new();
        win.update_pixmap(self.conn).unwrap();
        // Don't have to release everytime we bind
        unsafe {
            (self.glx_release_tex_image)(
                self.dpy,
                win.context.glxpixmap,
                GLX_FRONT_LEFT_EXT,
            );
        }
        win.context.glxpixmap = unsafe {
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

    pub fn update_pos(&self, win: &Window) {
        win.context.update_buffer(win);
    }

    pub fn update_texture(&self, win: &mut Window) {
        win.context.texture.bind();
        unsafe {
            (self.glx_bind_tex_image)(
                self.dpy,
                win.context.glxpixmap,
                GLX_FRONT_LEFT_EXT,
                std::ptr::null(),
            );
        }
        // Set texture parameters
        set_tex_params();
    }

    pub fn draw_window(&self, window: &Window) {
        window.context.vbo.bind();
        // TODO: OpenGl 4.3 has glBindVertexBuffers
        VertexArray::attrib_pointer(0, 2, 4, 0);
        VertexArray::attrib_pointer(1, 2, 4, 2);
        window.context.texture.bind();
        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            // TODO: check for performance impact of this line
            //setup::check_gl_error();
        }
    }

    pub fn render(&self) {
        unsafe {
            glXSwapBuffers(self.dpy, self.target_win);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            // draw root window
            self.root_data_vbo.bind();
            VertexArray::attrib_pointer(0, 2, 4, 0);
            VertexArray::attrib_pointer(1, 2, 4, 2);
            self.root_texture.bind();
            gl::DrawElements(
                gl::TRIANGLES,
                6,
                gl::UNSIGNED_INT,
                std::ptr::null(),
            );
            // TODO: check for performance impact of this line
            //setup::check_gl_error();
        }
    }
}

impl<'a> Drop for Opengl<'a> {
    fn drop(&mut self) {
        unsafe { glXDestroyContext(self.dpy, self.ctx) };
        self.conn.flush();
    }
}

#[derive(Default)]
pub struct BackendContext {
    pub glxpixmap: c_ulong,
    pub texture: Texture,
    pub vbo: Buffer,
}

impl BackendContext {
    pub fn load_buffer(&self, window: &Window) {
        // TODO: Look into geometry shader
        self.vbo.bind();
        self.vbo.load_data(&formatted_win_data(window));
    }
    pub fn update_buffer(&self, window: &Window) {
        // TODO: Look into geometry shader
        self.vbo.bind();
        self.vbo.update_data(&formatted_win_data(window));
    }
}

fn set_tex_params() {
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

fn formatted_win_data(window: &Window) -> [f32; 16] {
    [
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
    ]
}
