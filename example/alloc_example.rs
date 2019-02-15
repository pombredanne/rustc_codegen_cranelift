#![feature(start, box_syntax, alloc_system, core_intrinsics, trusted_len, exact_size_is_empty, ptr_offset_from, slice_partition_dedup, toowned_clone_into, raw_vec_internals, alloc, alloc_error_handler, dropck_eyepatch, specialization)]
#![no_std]

extern crate alloc;
extern crate alloc_system;

use alloc::prelude::Box;

use alloc_system::System;

#[global_allocator]
static ALLOC: System = System;

#[link(name = "c")]
extern "C" {
    fn puts(s: *const u8);
    fn printf(s: *const i8, ...);
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::intrinsics::abort();
    }
}

#[alloc_error_handler]
fn alloc_error_handler(_: alloc::alloc::Layout) -> ! {
    unsafe {
        core::intrinsics::abort();
    }
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    let mut vec = Vec::new();
    vec.push(CString::new("abc"));
    vec.push(CString::new("bcd"));
    for i in 0..vec.len() {
        unsafe { printf("iter el %d = %p len %d\n\0" as *const str as *const i8, i, vec[i].inner.as_ptr() as *const u8, vec[i].inner.len()); }
    }
    core::mem::drop(vec);

    0
}


use core::intrinsics::assume;
use core::ptr;
use core::slice::{self, SliceIndex};
use core::fmt;
use core::ops::{self, IndexMut, Index};
use alloc::raw_vec::RawVec;
use core::mem;

struct Vec<T> {
    buf: RawVec<T>,
    len: usize,
}



impl<T> Vec<T> {
    #[inline]
    const fn new() -> Vec<T> {
        Vec {
            buf: RawVec::new(),
            len: 0,
        }
    }

    #[inline]
    fn with_capacity(capacity: usize) -> Vec<T> {
        Vec {
            buf: RawVec::with_capacity(capacity),
            len: 0,
        }
    }

    fn reserve(&mut self, additional: usize) {
        self.buf.reserve(self.len, additional);
    }

    fn reserve_exact(&mut self, additional: usize) {
        self.buf.reserve_exact(self.len, additional);
    }

    fn shrink_to_fit(&mut self) {
        if self.buf.cap() != self.len {
            self.buf.shrink_to_fit(self.len);
        }
    }

    fn into_boxed_slice(mut self) -> Box<[T]> {
        unsafe {
            self.shrink_to_fit();
            let buf = ptr::read(&self.buf);
            mem::forget(self);
            buf.into_box()
        }
    }

    #[inline]
    unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.buf.cap());

        self.len = new_len;
    }

    #[inline]
    fn push(&mut self, value: T) {
        // This will panic or abort if we would allocate > isize::MAX bytes
        // or if the length increment would overflow for zero-sized types.
        if self.len == self.buf.cap() {
            self.reserve(1);
        }
        unsafe {
            let end = self.as_mut_ptr().add(self.len);
            ptr::write(end, value);
            self.len += 1;
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }
}

impl<T: Clone> Vec<T> {
    fn extend_from_slice(&mut self, other: &[T]) {
        self.extend_desugared(other.iter().cloned())
    }
}

// Common trait implementations for Vec

impl<T, I: SliceIndex<[T]>> Index<I> for Vec<T> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for Vec<T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<T> ops::Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            let p = self.buf.ptr();
            assume(!p.is_null());
            slice::from_raw_parts(p, self.len)
        }
    }
}

impl<T> ops::DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            let ptr = self.buf.ptr();
            assume(!ptr.is_null());
            slice::from_raw_parts_mut(ptr, self.len)
        }
    }
}

impl<T> Vec<T> {
    fn extend_desugared<I: Iterator<Item = T>>(&mut self, mut iterator: I) {
        // This is the case for a general iterator.
        //
        // This function should be the moral equivalent of:
        //
        //      for item in iterator {
        //          self.push(item);
        //      }
        while let Some(element) = iterator.next() {
            let len = self.len();
            if len == self.buf.cap() {
                let (lower, _) = iterator.size_hint();
                self.reserve(lower.saturating_add(1));
            }
            unsafe {
                ptr::write(self.get_unchecked_mut(len), element);
                // NB can't overflow since we would have had to alloc the address space
                self.set_len(len + 1);
            }
        }
    }
}

unsafe impl<#[may_dangle] T> Drop for Vec<T> {
    fn drop(&mut self) {
        unsafe {
            // use drop for [T]
            ptr::drop_in_place(&mut self[..]);
        }
        // RawVec handles deallocation
    }
}

impl<T: fmt::Debug> fmt::Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

mod hack {
    pub(crate) fn to_vec<T>(s: &[T]) -> crate::Vec<T>
        where T: Clone
    {
        let mut vector = crate::Vec::with_capacity(s.len());
        vector.extend_from_slice(s);
        vector
    }
}



struct CString {
    inner: Box<[u8]>,
}

impl CString {
    fn new(t: &str) -> CString {
        let mut v: Vec<u8> = hack::to_vec(t.as_bytes());
        v.reserve_exact(1);
        v.push(0);
        let inner = v.into_boxed_slice();
        unsafe { printf("cstring new %p len %d\n\0" as *const str as *const i8, inner.as_ptr() as *const u8, inner.len()); }
        CString { inner }
    }
}

impl Drop for CString {
    #[inline]
    fn drop(&mut self) {
        unsafe { printf("cstring drop %p len %d\n\0" as *const str as *const i8, self.inner.as_ptr() as *const u8, self.inner.len()); }
        //unsafe { printf("cstring drop get_unchecked_mut(0) = %p\n\0" as *const str as *const i8, self.inner.get_unchecked_mut(0)); }
        unsafe { *self.inner.get_unchecked_mut(0) = 0; }
    }
}

