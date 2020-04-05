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

// type of glxCreateContextAttribs extension
type GlXCreateContextAttribsARBProc = unsafe extern "C" fn(
    dpy: *mut xlib::Display,
    fbc: GLXFBConfig,
    share_context: GLXContext,
    direct: xlib::Bool,
    attribs: *const c_int,
) -> GLXContext;

pub fn init(conn: &xcb::Connection, screens: i32, win: xcb::Window) {
    if glx_dec_version(conn.get_raw_dpy()) < 13 {
        panic!("glx-1.3 is not supported");
    }

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
    );

    let setup = conn.get_setup();

    conn.flush();

    let glx_exts = unsafe {
        CStr::from_ptr(glXQueryExtensionsString(conn.get_raw_dpy(), screens))
            .to_str()
            .unwrap()
    };

    if !check_glx_extension(&glx_exts, "GLX_ARB_create_context") {
        panic!("could not find GLX extension GLX_ARB_create_context");
    }

    // with glx, no need of a current context is needed to load symbols
    // otherwise we would need to create a temporary legacy GL context
    // for loading symbols (at least glXCreateContextAttribsARB)
    let glx_create_context_attribs: GlXCreateContextAttribsARBProc = unsafe {
        std::mem::transmute(load_gl_func("glXCreateContextAttribsARB"))
    };

    // loading all other symbols
    unsafe { gl::load_with(|n| load_gl_func(&n)) };

    if !gl::GenVertexArrays::is_loaded() {
        panic!("no GL3 support available!");
    }

    // installing an event handler to check if error is generated
    unsafe { ctx_error_occurred = false };
    let old_handler =
        unsafe { xlib::XSetErrorHandler(Some(ctx_error_handler)) };

    let context_attribs: [c_int; 5] = [
        GLX_CONTEXT_MAJOR_VERSION_ARB as c_int,
        3,
        GLX_CONTEXT_MINOR_VERSION_ARB as c_int,
        0,
        0,
    ];
    let ctx = unsafe {
        glx_create_context_attribs(
            conn.get_raw_dpy(),
            fbc,
            null_mut(),
            xlib::True,
            &context_attribs[0] as *const c_int,
        )
    };

    conn.flush();
    unsafe { xlib::XSetErrorHandler(std::mem::transmute(old_handler)) };

    unsafe {
        if ctx.is_null() || ctx_error_occurred {
            panic!("error when creating gl-3.0 context");
        }
    }

    unsafe {
        if glXIsDirect(conn.get_raw_dpy(), ctx) == 0 {
            panic!("obtained indirect rendering context")
        }
    }

    unsafe {
        glXMakeCurrent(conn.get_raw_dpy(), win as xlib::XID, ctx);
        gl::ClearColor(0.5f32, 0.5f32, 1.0f32, 1.0f32);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::Flush();
        check_gl_error();
        glXSwapBuffers(conn.get_raw_dpy(), win as xlib::XID);
        glXMakeCurrent(conn.get_raw_dpy(), 0, null_mut());
    }
    conn.flush();

    // only to make sure that rs_client generate correct names for DRI2
    // (used to be "*_DRI_2_*")
    // should be in a "compile tests" section instead of example
    let _ = xcb::ffi::dri2::XCB_DRI2_ATTACHMENT_BUFFER_ACCUM;

    unsafe { glXDestroyContext(conn.get_raw_dpy(), ctx) };

    conn.flush();
}

fn get_glxfbconfig(
    dpy: *mut xlib::Display,
    screens: i32,
    visual_attribs: &[i32],
) -> GLXFBConfig {
    unsafe {
        let mut fbcount: c_int = 0;
        let fbcs = glXChooseFBConfig(
            dpy,
            screens,
            visual_attribs.as_ptr(),
            &mut fbcount as *mut c_int,
        );

        if fbcount == 0 {
            panic!("could not find compatible fb config");
        }
        // we pick the first from the list
        let fbc = *fbcs;
        xlib::XFree(fbcs as *mut c_void);
        fbc
    }
}

// returns the glx version in a decimal form
// eg. 1.3  => 13
fn glx_dec_version(dpy: *mut xlib::Display) -> i32 {
    let mut maj: c_int = 0;
    let mut min: c_int = 0;
    unsafe {
        if glXQueryVersion(dpy, &mut maj as *mut c_int, &mut min as *mut c_int)
            == 0
        {
            panic!("cannot get glx version");
        }
    }
    (maj * 10 + min) as i32
}

/// Chexks if a given glx extension exists in extension query string
fn check_glx_extension(glx_exts: &str, ext_name: &str) -> bool {
    for glx_ext in glx_exts.split(" ") {
        if glx_ext == ext_name {
            return true;
        }
    }
    false
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
