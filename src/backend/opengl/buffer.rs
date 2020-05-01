extern crate gl;

#[derive(Default)]
pub struct Buffer {
    id: gl::types::GLuint,
}

impl Buffer {
    pub fn new() -> Buffer {
        let mut id: gl::types::GLuint = 0;
        unsafe { gl::GenBuffers(1, &mut id) }
        Buffer { id }
    }
    pub fn bind(&self) {
        unsafe { gl::BindBuffer(gl::ARRAY_BUFFER, self.id) }
    }
    pub fn load_data<T>(&self, data: &[T]) {
        unsafe {
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (data.len() * std::mem::size_of::<T>())
                    as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );
        }
    }
    pub fn unbind() {
        unsafe { gl::BindBuffer(gl::ARRAY_BUFFER, 0) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.id);
        }
    }
}

pub struct VertexArray {
    id: gl::types::GLuint,
}

impl VertexArray {
    pub fn new() -> VertexArray {
        let mut id: gl::types::GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        VertexArray { id }
    }
    pub fn bind(&self) {
        unsafe {
            gl::BindVertexArray(self.id);
        }
    }
    pub fn enable(location: u32) {
        unsafe {
            gl::EnableVertexAttribArray(location);
        }
    }
    pub fn attrib_pointer(
        index: u32,
        size: i32,
        stride: usize,
        offset: usize,
    ) {
        let stride = (stride * std::mem::size_of::<f32>()) as gl::types::GLint;
        let offset =
            (offset * std::mem::size_of::<f32>()) as *const gl::types::GLvoid;
        unsafe {
            gl::VertexAttribPointer(
                index,
                size,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset,
            );
        }
    }
    pub fn unbind() {
        unsafe {
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for VertexArray {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &mut self.id);
        }
    }
}

pub struct ElementBuffer {
    id: gl::types::GLuint,
}

impl ElementBuffer {
    pub fn new() -> ElementBuffer {
        let mut id: gl::types::GLuint = 0;
        unsafe { gl::GenBuffers(1, &mut id) }
        ElementBuffer { id }
    }
    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
        }
    }
    pub fn load_data<T>(&self, data: &[T]) {
        unsafe {
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (data.len() * std::mem::size_of::<T>())
                    as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }
}

impl Drop for ElementBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.id);
        }
    }
}
