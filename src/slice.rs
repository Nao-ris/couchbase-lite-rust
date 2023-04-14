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

use crate::{
    CblRef,
    c_api::{FLSlice, FLSlice_Copy, FLSliceResult, _FLBuf_Release, _FLBuf_Retain, FLData_Dump},
};

use std::borrow::Cow;
use std::ffi::{CStr, c_void};
use std::fmt::{Debug, Formatter};
use std::ptr::{self, drop_in_place};
use std::str;

//////// SLICES

pub const NULL_SLICE: FLSlice = FLSlice {
    buf: ptr::null(),
    size: 0,
};

#[derive(Clone, Copy)]
pub struct Slice<T> {
    pub(crate) cbl_ref: FLSlice,
    _owner: T,
}

impl<T> CblRef for Slice<T> {
    type Output = FLSlice;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl<T> Slice<T> {
    pub(crate) const fn wrap(slice: FLSlice, owner: T) -> Self {
        Self {
            cbl_ref: slice,
            _owner: owner,
        }
    }

    pub fn as_byte_array(&self) -> Option<&[u8]> {
        unsafe { self.get_ref().as_byte_array() }
    }

    pub fn as_str(&self) -> Option<&str> {
        unsafe { self.get_ref().as_str() }
    }

    pub fn to_string(&self) -> Option<String> {
        unsafe { self.get_ref().to_string() }
    }

    pub fn to_vec(&self) -> Option<Vec<u8>> {
        unsafe { self.get_ref().to_vec() }
    }

    pub fn map<F, FT>(&self, f: F) -> Option<FT>
    where
        F: Fn(&FLSlice) -> FT,
    {
        self.get_ref().map(f)
    }
}

impl<T> Debug for Slice<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slice")
            .field("dump", unsafe {
                &FLData_Dump(self.get_ref()).to_string().unwrap_or_default()
            })
            .finish()
    }
}

pub fn from_str(s: &str) -> Slice<&str> {
    Slice::wrap(
        FLSlice {
            buf: s.as_ptr().cast::<c_void>(),
            size: s.len(),
        },
        s,
    )
}

pub fn from_bytes(s: &[u8]) -> Slice<&[u8]> {
    Slice::wrap(
        FLSlice {
            buf: s.as_ptr().cast::<c_void>(),
            size: s.len(),
        },
        s,
    )
}

impl FLSlice {
    // A slice may be null, so in Rust terms it's an Option.
    pub(crate) unsafe fn as_byte_array<'a>(&self) -> Option<&'a [u8]> {
        if !self {
            return None;
        }
        return Some(std::slice::from_raw_parts(self.buf.cast::<u8>(), self.size));
    }

    pub(crate) unsafe fn as_str<'a>(&self) -> Option<&'a str> {
        match self.as_byte_array() {
            None => None,
            Some(b) => str::from_utf8(b).ok(),
        }
    }

    pub(crate) unsafe fn to_string(self) -> Option<String> {
        self.as_str().map(std::string::ToString::to_string)
    }

    pub(crate) unsafe fn to_vec(self) -> Option<Vec<u8>> {
        self.as_byte_array().map(std::borrow::ToOwned::to_owned)
    }

    pub(crate) fn map<F, T>(&self, f: F) -> Option<T>
    where
        F: Fn(&Self) -> T,
    {
        if !self {
            return Some(f(self));
        }
        None
    }
}

impl std::ops::Not for &FLSlice {
    type Output = bool;
    fn not(self) -> bool {
        self.buf.is_null()
    }
}

impl std::ops::Not for FLSlice {
    type Output = bool;
    fn not(self) -> bool {
        self.buf.is_null()
    }
}

impl FLSliceResult {
    pub fn null() -> FLSliceResult {
        let s = FLSlice {
            buf: ptr::null(),
            size: 0,
        };
        unsafe { FLSlice_Copy(s) }
    }

    pub const fn as_slice(&self) -> FLSlice {
        FLSlice {
            buf: self.buf,
            size: self.size,
        }
    }

    // Consumes & releases self
    pub unsafe fn to_string(self) -> Option<String> {
        self.as_slice().to_string()
    }

    // Consumes & releases self
    pub unsafe fn to_vec(self) -> Option<Vec<u8>> {
        self.as_slice().to_vec()
    }
}

impl Clone for FLSliceResult {
    fn clone(&self) -> Self {
        unsafe { _FLBuf_Retain(self.buf) };
        Self {
            buf: self.buf,
            size: self.size,
        }
    }
}

impl Drop for FLSliceResult {
    fn drop(&mut self) {
        unsafe { _FLBuf_Release(self.buf) };
    }
}

//////// C STRINGS

// Convenience to convert a raw `char*` to an unowned `&str`
pub unsafe fn to_str<'a>(cstr: *const ::std::os::raw::c_char) -> Cow<'a, str> {
    CStr::from_ptr(cstr).to_string_lossy()
}

// Convenience to convert a raw `char*` to an owned String
pub unsafe fn to_string(cstr: *const ::std::os::raw::c_char) -> String {
    to_str(cstr).to_string()
}

pub unsafe fn free_cstr(cstr: *const ::std::os::raw::c_char) {
    drop_in_place(cstr as *mut ::std::os::raw::c_char);
}
