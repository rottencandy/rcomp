extern crate gl;
extern crate x11;

use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};
use std::ptr::null_mut;

pub use x11::{glx::*, xlib};

// Minimum reuqired version for glxCreateContextAttribs extension
const GLX_CONTEXT_MAJOR_VERSION_ARB: u32 = 0x2091;
const GLX_CONTEXT_MINOR_VERSION_ARB: u32 = 0x2092;

#[allow(non_upper_case_globals)]
static mut ctx_error_occurred: bool = false;

pub fn verify_extensions(
    conn: &xcb::Connection,
    screens: i32,
) -> Result<(), &str> {
    if glx_dec_version(conn.get_raw_dpy())? < 13 {
        return Err("glx-1.3 is not supported");
    }

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

pub fn create_glx_context(
    conn: &xcb::Connection,
    fbc: GLXFBConfig,
) -> Result<GLXContext, &str> {
    // with glx, no need of a current context is needed to load symbols
    // otherwise we would need to create a temporary legacy GL context
    // for loading symbols (at least glXCreateContextAttribsARB)
    let glx_create_context_attribs: GlXCreateContextAttribsARBProc = unsafe {
        std::mem::transmute(load_gl_func("glXCreateContextAttribsARB"))
    };
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
    unsafe {
        let ctx = glx_create_context_attribs(
            conn.get_raw_dpy(),
            fbc,
            null_mut(),
            xlib::True,
            &context_attribs[0] as *const c_int,
        );
        if ctx.is_null() || ctx_error_occurred {
            return Err("error when creating gl-3.0 context");
        }
        if glXIsDirect(conn.get_raw_dpy(), ctx) == 0 {
            return Err("obtained indirect rendering context");
        }

        xlib::XSetErrorHandler(std::mem::transmute(old_handler));
        Ok(ctx)
    }
}

/// returns the glx version in a decimal form
/// eg. 1.3  => 13
pub fn glx_dec_version(dpy: *mut xlib::Display) -> Result<i32, &'static str> {
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
pub fn check_glx_extension(glx_exts: &str, ext_name: &str) -> bool {
    for glx_ext in glx_exts.split(' ') {
        if glx_ext == ext_name {
            return true;
        }
    }
    false
}

// type for glxCreateContextAttribs extension function
pub type GlXCreateContextAttribsARBProc = unsafe extern "C" fn(
    dpy: *mut xlib::Display,
    fbc: GLXFBConfig,
    share_context: GLXContext,
    direct: xlib::Bool,
    attribs: *const c_int,
) -> GLXContext;

// types for texture_from_pixmap extension functions
pub type GLXBindTexImageEXT = unsafe extern "C" fn(
    dpy: *mut xlib::Display,
    drawable: GLXDrawable,
    buffer: c_int,
    attribs: *const c_int,
);
pub type GLXReleaseTexImageEXT = unsafe extern "C" fn(
    dpy: *mut xlib::Display,
    drawable: GLXDrawable,
    buffer: c_int,
);

pub fn get_glxfbconfig(
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

pub unsafe fn load_gl_func(name: &str) -> *mut c_void {
    let cname = CString::new(name).unwrap();
    let ptr: *mut c_void =
        std::mem::transmute(glXGetProcAddress(cname.as_ptr() as *const u8));
    if ptr.is_null() {
        panic!("could not load {}", name);
    }
    ptr
}

pub unsafe extern "C" fn ctx_error_handler(
    _dpy: *mut xlib::Display,
    _ev: *mut xlib::XErrorEvent,
) -> i32 {
    ctx_error_occurred = true;
    0
}

pub unsafe fn check_gl_error() {
    let err = gl::GetError();
    if err != gl::NO_ERROR {
        eprintln!("Got gl error: {}", err);
    }
}
