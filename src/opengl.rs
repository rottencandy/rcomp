extern crate gl;
extern crate x11;

use x11::{glx::*, xlib};

use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};
use std::ptr::null_mut;

// Minimum reuqired version for glxCreateContextAttribs extension
const GLX_CONTEXT_MAJOR_VERSION_ARB: u32 = 0x2091;
const GLX_CONTEXT_MINOR_VERSION_ARB: u32 = 0x2092;

static mut ctx_error_occurred: bool = false;

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
        verify_extensions(conn, screens)?;
        // setup framebuffer context
        let fbc = get_glxfbconfig(
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
        // with glx, no need of a current context is needed to load symbols
        // otherwise we would need to create a temporary legacy GL context
        // for loading symbols (at least glXCreateContextAttribsARB)
        let glx_create_context_attribs: GlXCreateContextAttribsARBProc = unsafe {
            std::mem::transmute(load_gl_func("glXCreateContextAttribsARB"))
        };

        // load all function pointers
        unsafe { gl::load_with(|n| load_gl_func(&n)) };

        if !gl::GenVertexArrays::is_loaded() {
            return Err("no GL3 support available!");
        }
        // installing an event handler to check if error is generated
        unsafe { ctx_error_occurred = false };
        let old_handler =
            unsafe { xlib::XSetErrorHandler(Some(ctx_error_handler)) };

        let context_attribs: [c_int; 5] = [
            GLX_CONTEXT_MAJOR_VERSION_ARB as c_int,
            3,
            GLX_CONTEXT_MINOR_VERSION_ARB as c_int,
            3,
            0,
        ];
        let ctx = unsafe {
            let ctx = glx_create_context_attribs(
                conn.get_raw_dpy(),
                fbc,
                null_mut(),
                xlib::True,
                &context_attribs[0] as *const c_int,
            );
            conn.flush();
            if ctx.is_null() || ctx_error_occurred {
                return Err("error when creating gl-3.0 context");
            }
            if glXIsDirect(conn.get_raw_dpy(), ctx) == 0 {
                return Err("obtained indirect rendering context");
            }
            ctx
        };

        unsafe { xlib::XSetErrorHandler(std::mem::transmute(old_handler)) };
        Ok(Opengl {
            ctx: ctx,
            conn: conn,
            dpy: conn.get_raw_dpy(),
            draw_win: win as xlib::XID,
        })
    }

    pub fn draw(&self) {
        unsafe {
            glXMakeCurrent(self.dpy, self.draw_win, self.ctx);
            gl::ClearColor(0.3f32, 0.3f32, 0.4f32, 1.0f32);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Flush();
            check_gl_error();
            glXSwapBuffers(self.dpy, self.draw_win);
            glXMakeCurrent(self.dpy, 0, null_mut());
        }
        self.conn.flush();
    }
}

impl<'a> Drop for Opengl<'a> {
    fn drop(&mut self) {
        // only to make sure that rs_client generate correct names for DRI2
        // (used to be "*_DRI_2_*")
        // should be in a "compile tests" section instead of example
        let _ = xcb::ffi::dri2::XCB_DRI2_ATTACHMENT_BUFFER_ACCUM;
        unsafe { glXDestroyContext(self.dpy, self.ctx) };
        self.conn.flush();
    }
}

fn verify_extensions(
    conn: &xcb::Connection,
    screens: i32,
) -> Result<(), &str> {
    if glx_dec_version(conn.get_raw_dpy())? < 13 {
        return Err("glx-1.3 is not supported");
    }

    // verify extensions
    let glx_exts = unsafe {
        CStr::from_ptr(glXQueryExtensionsString(conn.get_raw_dpy(), screens))
            .to_str()
            .unwrap()
    };

    if !check_glx_extension(&glx_exts, "GLX_ARB_create_context") {
        return Err("could not find GLX extension GLX_ARB_create_context");
    }
    if !check_glx_extension(&glx_exts, "GLX_EXT_texture_from_pixmap") {
        return Err(
            "could not find GLX extension GLX_EXT_texture_from_pixmap",
        );
    }
    Ok(())
}

// returns the glx version in a decimal form
// eg. 1.3  => 13
fn glx_dec_version(dpy: *mut xlib::Display) -> Result<i32, &'static str> {
    let mut maj: c_int = 0;
    let mut min: c_int = 0;
    unsafe {
        if glXQueryVersion(dpy, &mut maj as *mut c_int, &mut min as *mut c_int)
            == 0
        {
            return Err("cannot get glx version");
        }
    }
    Ok((maj * 10 + min) as i32)
}

/// Checks if a given glx extension exists in extension query string
fn check_glx_extension(glx_exts: &str, ext_name: &str) -> bool {
    for glx_ext in glx_exts.split(" ") {
        if glx_ext == ext_name {
            return true;
        }
    }
    false
}

// type of glxCreateContextAttribs extension
type GlXCreateContextAttribsARBProc = unsafe extern "C" fn(
    dpy: *mut xlib::Display,
    fbc: GLXFBConfig,
    share_context: GLXContext,
    direct: xlib::Bool,
    attribs: *const c_int,
) -> GLXContext;

fn get_glxfbconfig(
    dpy: *mut xlib::Display,
    screens: i32,
    visual_attribs: &[i32],
) -> Result<GLXFBConfig, &str> {
    unsafe {
        let mut fbcount: c_int = 0;
        let fbcs = glXChooseFBConfig(
            dpy,
            screens,
            visual_attribs.as_ptr(),
            &mut fbcount as *mut c_int,
        );

        if fbcount == 0 {
            return Err("could not find compatible fb config");
        }
        // we pick the first from the list
        let fbc = *fbcs;
        xlib::XFree(fbcs as *mut c_void);
        Ok(fbc)
    }
}

unsafe fn load_gl_func(name: &str) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    let ptr: *mut c_void =
        std::mem::transmute(glXGetProcAddress(cname.as_ptr() as *const u8));
    if ptr.is_null() {
        panic!("could not load {}", name);
    }
    ptr
}

unsafe extern "C" fn ctx_error_handler(
    _dpy: *mut xlib::Display,
    _ev: *mut xlib::XErrorEvent,
) -> i32 {
    ctx_error_occurred = true;
    0
}

unsafe fn check_gl_error() {
    let err = gl::GetError();
    if err != gl::NO_ERROR {
        println!("got gl error {}", err);
    }
}

struct Shader {
    pub id: gl::types::GLuint,
}

impl Shader {
    pub fn from_source(
        source: &CStr,
        kind: gl::types::GLuint,
    ) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;
        Ok(Shader { id })
    }
    pub fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }
    pub fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

fn shader_from_source(
    source: &CStr,
    kind: gl::types::GLuint,
) -> Result<gl::types::GLuint, String> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }
    let mut success: gl::types::GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }
    if success == 0 {
        let mut buflen: gl::types::GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut buflen);
        }
        let error = create_whitespace_cstring_with_len(buflen as usize);
        unsafe {
            gl::GetShaderInfoLog(
                id,
                buflen,
                std::ptr::null_mut(),
                error.as_ptr() as *mut gl::types::GLchar,
            );
        }
        return Err(error.to_string_lossy().into_owned());
    }
    Ok(id)
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len as usize + 1);
    buffer.extend([b' '].iter().cycle().take(len as usize));
    unsafe { CString::from_vec_unchecked(buffer) }
}
