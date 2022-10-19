// Couchbase Lite main module
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

//#![allow(unused_imports)]
//#![allow(dead_code)]

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::copy_iterator)]
#![allow(clippy::missing_panics_doc)]

#[macro_use]
extern crate enum_primitive;

pub mod blob;
pub mod database;
pub mod document;
pub mod encryptable;
pub mod error;
pub mod fleece;
pub mod fleece_mutable;
pub mod index;
pub mod logging;
pub mod query;
pub mod replicator;
pub mod slice;

mod c_api;

use self::c_api::{
    CBLListenerToken, CBLRefCounted, CBL_DumpInstances, CBL_InstanceCount, CBL_Release, CBL_Retain,
    CBLListener_Remove,
};
#[cfg(target_os = "android")]
use self::c_api::{CBLError, CBLInitContext, CBL_Init};
#[cfg(target_os = "android")]
use std::ffi::CStr;

//////// RE-EXPORT:

pub use blob::*;
pub use database::*;
pub use document::*;
pub use error::*;
pub use fleece::*;
pub use fleece_mutable::*;
pub use query::*;
pub use replicator::*;

//////// TOP-LEVEL TYPES:

pub trait CblRef {
    type Output;
    fn get_ref(&self) -> Self::Output;
}

#[derive(Debug, Clone, Copy)]
/// A time value for document expiration. Defined as milliseconds since the Unix epoch (1/1/1970.)
pub struct Timestamp(pub i64);

pub struct Listener<T> {
    pub listener_token: ListenerToken,
    pub listener: T,
}

impl<T> Listener<T> {
    pub fn new(listener_token: ListenerToken, listener: T) -> Self {
        Self {
            listener_token,
            listener,
        }
    }
}

/// An opaque token representing a registered listener.
/// When this object is dropped, the listener function will not be called again.
pub struct ListenerToken {
    cbl_ref: *mut CBLListenerToken,
}

impl ListenerToken {
    pub(crate) const fn new(cbl_ref: *mut CBLListenerToken) -> Self {
        Self { cbl_ref }
    }
}

impl CblRef for ListenerToken {
    type Output = *mut CBLListenerToken;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Drop for ListenerToken {
    fn drop(&mut self) {
        unsafe { CBLListener_Remove(self.get_ref()) }
    }
}

//////// MISC. API FUNCTIONS

/** Returns the total number of Couchbase Lite objects. Useful for leak checking. */
pub fn instance_count() -> usize {
    unsafe { CBL_InstanceCount() as usize }
}

/** Logs the class and address of each Couchbase Lite object. Useful for leak checking.
@note  May only be functional in debug builds of Couchbase Lite. */
pub fn dump_instances() {
    unsafe { CBL_DumpInstances() }
}

//////// REFCOUNT SUPPORT (INTERNAL)

pub(crate) unsafe fn retain<T>(cbl_ref: *mut T) -> *mut T {
    CBL_Retain(cbl_ref.cast::<CBLRefCounted>()).cast::<T>()
}

pub(crate) unsafe fn release<T>(cbl_ref: *mut T) {
    CBL_Release(cbl_ref.cast::<CBLRefCounted>());
}

//////// ANDROID INIT

/** Application context information required for Android application to initialize before using
CouchbaseLite library. */
#[cfg(target_os = "android")]
pub struct AndroidInitContext {
    pub files_directory: &'static CStr, //< The directory where the opened database will be stored when a specific database directory is not specified in \ref CBLDatabaseConfiguration.
    //< Normally the path returned from Android Context's getFilesDir() method can be specified here unless different directory is desired.
    //< The specified fileDir must exist otherwise an error will be returend when calling \r CBL_Init().
    pub temp_directory: &'static CStr, //< The directory where the SQLite stores its temporary files.
                                       //< Normally the path returned from Android Context's getExternalFilesDir(String type) with a custom type such as "CBLTemp" can be specified here
                                       //< unless different directory is desired. The specified tempDir must exist otherwise an error will be returend when calling \r CBL_Init().
}

/** Initialize application context information for Android application. This function is required
   to be called the first time before using the CouchbaseLite library otherwise an error will be
   returned when calling CBLDatabase_Open to open a database. Call \r CBL_Init more than once will
   return an error.
   @param context  The application context information.
*/
#[cfg(target_os = "android")]
pub fn android_init(context: AndroidInitContext) -> Result<()> {
    let mut err = CBLError::default();
    unsafe {
        CBL_Init(
            CBLInitContext {
                filesDir: context.files_directory.as_ptr(),
                tempDir: context.temp_directory.as_ptr(),
            },
            &mut err,
        );
    }
    check_error(&err)
}
