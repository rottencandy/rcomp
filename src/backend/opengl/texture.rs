extern crate gl;

#[derive(Default)]
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
