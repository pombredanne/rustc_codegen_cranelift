#![feature(rustc_private)]

extern crate libc;

fn main() {
    let _ = vec![
        CString::new("abc"),
        CString::new("bcd"),
    ];
}

pub struct CString {
    inner: Box<[u8]>,
}

impl CString {
    pub fn new<T: Into<Vec<u8>>>(t: T) -> CString {
        let mut v: Vec<u8> = t.into();
        v.reserve_exact(1);
        v.push(0);
        CString { inner: v.into_boxed_slice() }
    }
}

impl Drop for CString {
    #[inline]
    fn drop(&mut self) {
        unsafe { libc::printf("cstring drop %p len %d\n\0" as *const str as *const i8, self.inner.as_ptr() as *const u8, self.inner.len()); }
        unsafe { libc::printf("cstring drop get_unchecked_mut(0) = %p\n\0" as *const str as *const i8, self.inner.get_unchecked_mut(0)); }
        unsafe { *self.inner.get_unchecked_mut(0) = 0; }
    }
}
