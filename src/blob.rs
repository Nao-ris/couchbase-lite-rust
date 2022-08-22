// Couchbase Lite Blob class
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
    CblRef, Database, Dict, FleeceReference, Result, Slot, check_io, check_ptr, failure, release,
    retain,
    slice::{from_bytes, from_str},
    c_api::{
        CBLBlob, CBLBlobReadStream, CBLBlobReader_Close, CBLBlobReader_Read, CBLBlobWriteStream,
        CBLBlobWriter_Close, CBLBlobWriter_Create, CBLBlobWriter_Write, CBLBlob_Content,
        CBLBlob_ContentType, CBLBlob_CreateWithData, CBLBlob_CreateWithStream, CBLBlob_Digest,
        CBLBlob_Length, CBLBlob_OpenContentStream, CBLBlob_Properties, CBLError, FLDict_GetBlob,
        FLSlot_SetBlob, FLValue_AsDict,
    },
};

use std::ffi::c_void;
use std::marker::PhantomData;

/** A binary attachment to a Document. */
pub struct Blob {
    pub(crate) cbl_ref: *const CBLBlob,
}

impl CblRef for Blob {
    type Output = *const CBLBlob;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Blob {
    //////// CREATION

    /** Creates a new blob, given its contents as a byte array. */
    pub fn new_from_data(data: &[u8], content_type: &str) -> Self {
        unsafe {
            let blob = CBLBlob_CreateWithData(
                from_str(content_type).get_ref(),
                from_bytes(data).get_ref(),
            );
            Self { cbl_ref: blob }
        }
    }

    /** Creates a new blob from data that has has been written to a [`Writer`].
    You should then add the blob to a document as a property, using [`Slot::put_blob`]. */
    pub fn new_from_stream(mut stream: BlobWriter, content_type: &str) -> Self {
        unsafe {
            let blob = CBLBlob_CreateWithStream(from_str(content_type).get_ref(), stream.get_ref());
            stream.stream_ref = std::ptr::null_mut(); // stop `drop` from closing the stream
            Self { cbl_ref: blob }
        }
    }

    // called by FleeceReference::as_blob()
    pub(crate) fn from_value<V: FleeceReference>(value: &V) -> Option<Self> {
        unsafe {
            let blob = FLDict_GetBlob(FLValue_AsDict(value._fleece_ref()));
            if blob.is_null() {
                None
            } else {
                Some(Self { cbl_ref: blob })
            }
        }
    }

    //////// ACCESSORS

    /** The length of the content data in bytes. */
    pub fn length(&self) -> u64 {
        unsafe { CBLBlob_Length(self.get_ref()) }
    }

    /** The unique digest of the blob: A base64-encoded SHA-1 digest of its data. */
    pub fn digest(&self) -> &str {
        unsafe { CBLBlob_Digest(self.get_ref()).as_str().unwrap() }
    }

    /** The MIME type assigned to the blob, if any. */
    pub fn content_type(&self) -> Option<&str> {
        unsafe { CBLBlob_ContentType(self.get_ref()).as_str() }
    }

    /** The blob's metadata properties as a dictionary. */
    pub fn properties(&self) -> Dict {
        unsafe { Dict::new(CBLBlob_Properties(self.get_ref())) }
    }

    //////// READING:

    /** Reads the blob's contents into memory and returns them as a byte array.
    This can potentially allocate a lot of memory! */
    pub fn load_content(&self) -> Result<Vec<u8>> {
        unsafe {
            let mut err = CBLError::default();
            let content = CBLBlob_Content(self.get_ref(), &mut err).to_vec();
            content.map_or_else(|| failure(err), Ok)
        }
    }

    /** Opens a stream for reading a blob's content from disk. */
    pub fn open_content(&self) -> Result<BlobReader> {
        check_ptr(
            |err| unsafe { CBLBlob_OpenContentStream(self.get_ref(), err) },
            |stream| BlobReader {
                blob: self,
                stream_ref: stream,
            },
        )
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        unsafe {
            release(self.get_ref() as *mut CBLBlob);
        }
    }
}

impl Clone for Blob {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                cbl_ref: retain(self.get_ref() as *mut CBLBlob),
            }
        }
    }
}

//////// BLOB ADDITIONS FOR ARRAY / DICT:

impl Slot<'_> {
    /** Stores a Blob reference in an Array or Dict. This is how you add a Blob to a Document. */
    pub fn put_blob(self, blob: &mut Blob) {
        unsafe { FLSlot_SetBlob(self.get_ref(), blob.get_ref() as *mut CBLBlob) }
    }
}

//////// BLOB READER

/** A stream for reading Blob conents. */
pub struct BlobReader<'r> {
    pub blob: &'r Blob,
    stream_ref: *mut CBLBlobReadStream,
}

impl<'r> CblRef for BlobReader<'r> {
    type Output = *mut CBLBlobReadStream;
    fn get_ref(&self) -> Self::Output {
        self.stream_ref
    }
}

impl<'r> std::io::Read for BlobReader<'r> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            check_io(|err| {
                CBLBlobReader_Read(
                    self.get_ref(),
                    buf.as_mut_ptr().cast::<c_void>(),
                    buf.len(),
                    err,
                )
            })
        }
    }
}

impl<'r> Drop for BlobReader<'r> {
    fn drop(&mut self) {
        unsafe {
            CBLBlobReader_Close(self.get_ref());
        }
    }
}

//////// BLOB WRITER

/** A stream for writing data that will become a Blob's contents.
After you're done writing the data, call [`Blob::new_from_stream`],
then add the Blob to a document property via [`Slot::put_blob`]. */
pub struct BlobWriter<'d> {
    stream_ref: *mut CBLBlobWriteStream,
    db: PhantomData<&'d mut Database>,
}

impl<'d> CblRef for BlobWriter<'d> {
    type Output = *mut CBLBlobWriteStream;
    fn get_ref(&self) -> Self::Output {
        self.stream_ref
    }
}

impl<'d> BlobWriter<'d> {
    pub fn new(db: &'d mut Database) -> Result<BlobWriter<'d>> {
        unsafe {
            let db_ref = db.get_ref();
            check_ptr(
                |err| CBLBlobWriter_Create(db_ref, err),
                move |stream| BlobWriter {
                    stream_ref: stream,
                    db: PhantomData,
                },
            )
        }
    }
}

impl<'r> std::io::Write for BlobWriter<'r> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        unsafe {
            check_io(|err| {
                let ok = CBLBlobWriter_Write(
                    self.get_ref(),
                    data.as_ptr().cast::<c_void>(),
                    data.len(),
                    err,
                );
                if ok {
                    data.len() as i32
                } else {
                    -1
                }
            })
        }
    }

    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

impl<'r> Drop for BlobWriter<'r> {
    fn drop(&mut self) {
        unsafe { CBLBlobWriter_Close(self.get_ref()) }
    }
}
