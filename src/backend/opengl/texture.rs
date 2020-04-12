extern crate gl;

use super::setup;

pub struct Texture {
    pub id: gl::types::GLuint,
    is_pixmap: bool,
}

impl Texture {
    pub fn new() -> Texture {
        let mut id: gl::types::GLuint = 0;
        unsafe { gl::GenTextures(1, &mut id) }
        Texture { id, is_pixmap: false }
    }
    pub fn from_pixmap(
        pixmap: xcb::Pixmap,
        dpy: *mut setup::xlib::Display,
        fbc: setup::GLXFBConfig,
    ) -> Texture {
        let glxpixmap = unsafe {
            setup::glXCreatePixmap(
                dpy,
                fbc,
                pixmap as u64,
                [0x20d6, 0x20dc, 0x20d5, 0x20da, 0x0000].as_ptr(),
            )
        };
        let glx_bind_tex_image: setup::GLXBindTexImageEXT = unsafe {
            std::mem::transmute(setup::load_gl_func("glXBindTexImageEXT"))
        };
        let glx_release_tex_image: setup::GLXReleaseTexImageEXT = unsafe {
            std::mem::transmute(setup::load_gl_func("glXReleaseTexImageEXT"))
        };
        let mut texture = Texture::new();
        texture.bind();
        texture.is_pixmap = true;
        unsafe {
            glx_bind_tex_image(dpy, glxpixmap, 0x20de, std::ptr::null());
        }
        texture
    }
    pub fn bind(&self) {
        unsafe { gl::BindTexture(gl::TEXTURE_2D, self.id) }
    }
    pub fn set_active(i: u32) {
        //TODO: proc macros
        unsafe {
            match i {
                0 => gl::ActiveTexture(gl::TEXTURE0),
                1 => gl::ActiveTexture(gl::TEXTURE1),
                2 => gl::ActiveTexture(gl::TEXTURE2),
                3 => gl::ActiveTexture(gl::TEXTURE3),
                _ => {}
            }
        };
    }
    pub fn unbind() {
        unsafe { gl::BindTexture(gl::TEXTURE_2D, 0) }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &mut self.id);
        }
    }
}
