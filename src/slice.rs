// Internal API for working with FLSlice, FLSliceResult, and C strings
//
// Copyright (c) 2020 Couchbase, Inc All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use super::c_api::*;

// use std::borrow::Cow;
use std::ffi::c_void;
// use std::ffi::CStr;
use std::ptr;
use std::str;


//////// SLICES


pub const NULL_SLICE : FLSlice = FLSlice{buf: ptr::null(), size: 0};

#[derive(Clone, Copy)]
pub struct Slice<T> {
    pub(crate) _ref: FLSlice,
    _owner: T,
}

impl<T> Slice<T> {
    pub const fn wrap(slice: FLSlice, _owner: T) -> Self {
        Self {
            _ref: slice,
            _owner,
        }
    }

    pub fn as_byte_array(&self) -> Option<&[u8]> {
        unsafe { self._ref.as_byte_array() }
    }

    pub fn as_str(&self) -> Option<&str> {
        unsafe { self._ref.as_str() }
    }

    pub fn to_string(&self) -> Option<String> {
        unsafe { self._ref.to_string() }
    }

    pub fn to_vec(&self) -> Option<Vec<u8>> {
        unsafe{ self._ref.to_vec() }
    }

    pub fn map<F, FT>(&self, f : F) -> Option<FT>
        where F: Fn(&FLSlice)->FT
    {
        self._ref.map(f)
    }
}

pub fn as_slice(s: &str) -> Slice<&str> {
    Slice::wrap(FLSlice{buf: s.as_ptr() as *const c_void,
        size: s.len() as u64 },s)
}

pub fn bytes_as_slice(s: &[u8]) -> Slice<&[u8]> {
     Slice::wrap(FLSlice{buf: s.as_ptr() as *const c_void,
        size: s.len() as u64}, s)
}

impl FLSlice {
    // A slice may be null, so in Rust terms it's an Option.

    pub unsafe fn as_byte_array<'a>(&self) -> Option<&'a [u8]> {
        if !self { return None }
        return Some(std::slice::from_raw_parts(self.buf as *const u8, self.size as usize))
    }

    pub unsafe fn as_str<'a>(&self) -> Option<&'a str> {
        match self.as_byte_array() {
            None    => None,
            Some(b) => { str::from_utf8(b).ok() }
        }
    }
    pub unsafe fn to_string(&self) -> Option<String> {
        return self.as_str().map(|s| s.to_string());
    }

    pub unsafe fn to_vec(&self) -> Option<Vec<u8>> {
        return self.as_byte_array().map(|a| a.to_owned());
    }

    pub fn map<F, T>(&self, f : F) -> Option<T>
        where F: Fn(&FLSlice)->T
    {
        if !self {None} else {Some(f(self))}
    }
}

impl std::ops::Not for &FLSlice {
    type Output = bool;
    fn not(self) -> bool {self.buf.is_null()}
}

impl std::ops::Not for FLSlice {
    type Output = bool;
    fn not(self) -> bool {self.buf.is_null()}
}

impl FLSliceResult {
    pub fn as_slice(&self) -> FLSlice {
        return FLSlice{buf: self.buf, size: self.size};
    }

    // pub unsafe fn retain(&mut self) {
    //     _FLBuf_Retain(self.buf);
    // }

    // It's not possible to implement Drop for FLSliceResult, because the generated interface
    // makes it implement Copy. So it has to be released by hand.
    pub unsafe fn release(&mut self) {
        _FLBuf_Release(self.buf);
    }

    // Consumes & releases self
    pub unsafe fn to_string(mut self) -> Option<String> {
        let str = self.as_slice().to_string();
        self.release();
        return str;
    }

    // Consumes & releases self
    pub unsafe fn to_vec(mut self) -> Option<Vec<u8>> {
        let vec = self.as_slice().to_vec();
        self.release();
        return vec;
    }
}


//////// C STRINGS


// Convenience to convert a raw `char*` to an unowned `&str`
// pub unsafe fn to_str<'a>(cstr: *const ::std::os::raw::c_char) -> Cow<'a, str> {
//     return CStr::from_ptr(cstr).to_string_lossy()
// }


// Convenience to convert a raw `char*` to an owned String
// pub unsafe fn to_string(cstr: *const ::std::os::raw::c_char) -> String {
//     return to_str(cstr).to_string();
// }

/*
pub(crate) unsafe fn free_cstr(cstr: *const ::std::os::raw::c_char) {
    todo!(); // Implement this by calling `free()`
}
*/
