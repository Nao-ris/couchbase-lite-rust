// Couchbase Lite document API
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

use super::*;
use super::slice::*;
use super::c_api::*;


/** An in-memory copy of a document. */
pub struct Document {
    _ref: *mut CBLDocument,
    has_ownership: bool,
}


//////// DATABASE'S DOCUMENT API:


/** Conflict-handling options when saving or deleting a document. */
pub enum ConcurrencyControl {
    LastWriteWins  = kCBLConcurrencyControlLastWriteWins as isize,
    FailOnConflict = kCBLConcurrencyControlFailOnConflict as isize
}

pub type SaveConflictHandler = fn(&mut Document, &Document) -> bool;
#[no_mangle]
unsafe extern "C" fn c_save_conflict_handler(
    context: *mut ::std::os::raw::c_void,
    user_doc: *mut CBLDocument,
    existing_doc: *const CBLDocument,
) -> bool {
    let callback: SaveConflictHandler = std::mem::transmute(context);

    callback(&mut Document { _ref: user_doc, has_ownership: false }, &Document { _ref: existing_doc as *mut CBLDocument, has_ownership: false })
}

pub type ChangeListener = fn(&Database, &str);
#[no_mangle]
unsafe extern "C" fn c_document_change_listener(
    context: *mut ::std::os::raw::c_void,
    db: *const CBLDatabase,
    c_doc_id: FLString,
) {
    let callback: ChangeListener = std::mem::transmute(context);

    let database = Database {
        _ref: db as *mut CBLDatabase,
        has_ownership: false,
    };

    callback(&database, c_doc_id.to_string().unwrap().as_ref());
}

impl Database {
    /** Reads a document from the database. Each call to this function returns a new object
        containing the document's current state. */
    pub fn get_document(&self, id: &str) -> Result<Document> {
        unsafe {
            // we always get a mutable CBLDocument,
            // since Rust doesn't let us have MutableDocument subclass.
            let mut error = CBLError::default();
            let doc = CBLDatabase_GetMutableDocument(self._ref, as_slice(id), &mut error);
            if doc.is_null() {
                if error.code != 0 {
                    return failure(error);
                } else {
                    return Err(Error::cbl_error(CouchbaseLiteError::NotFound));
                }
            }
            return Ok(Document{_ref: doc, has_ownership: true});
        }
    }

    /** Saves a new or modified document to the database.
        If a conflicting revision has been saved since `doc` was loaded, the `concurrency`
        parameter specifies whether the save should fail, or the conflicting revision should
        be overwritten with the revision being saved.
        If you need finer-grained control, call `save_document_resolving` instead. */
    pub fn save_document(&mut self,
                         doc: &mut Document,
                         concurrency: ConcurrencyControl)
                         -> Result<()>
    {
        let c_concurrency = concurrency as u8;
        unsafe {
            return check_bool(|error| CBLDatabase_SaveDocumentWithConcurrencyControl(
                                            self._ref, doc._ref, c_concurrency, error))
        }
    }

    /** Saves a new or modified document to the database. This function is the same as
        `save_document`, except that it allows for custom conflict handling in the event
        that the document has been updated since `doc` was loaded. */
    pub fn save_document_resolving(&mut self,
                                   doc: &mut Document,
                                   conflict_handler: SaveConflictHandler)
                                   -> Result<Document>
    {
        unsafe {
            let callback: *mut ::std::os::raw::c_void = std::mem::transmute(conflict_handler);
            match check_bool(|error| CBLDatabase_SaveDocumentWithConflictHandler(
                self._ref, doc._ref, Some(c_save_conflict_handler), callback, error)) {
                Ok(_) => Ok(doc.to_owned()),
                Err(err) => Err(err)
            }
        }
    }

    pub fn delete_document(&mut self, doc: &Document, concurrency: ConcurrencyControl) -> Result<()> {
        let c_concurrency = concurrency as u8;
        unsafe {
            return check_bool(|error| CBLDatabase_DeleteDocumentWithConcurrencyControl(self._ref, doc._ref, c_concurrency, error));
        }
    }

    pub fn purge_document(&mut self, doc: &Document) -> Result<()> {
        unsafe {
            return check_bool(|error| CBLDatabase_PurgeDocument(self._ref, doc._ref, error));
        }
    }

    pub fn purge_document_by_id(&mut self, id: &str) -> Result<()> {
        unsafe {
            return check_bool(|error| CBLDatabase_PurgeDocumentByID(self._ref, as_slice(id), error));
        }
    }

    /** Returns the time, if any, at which a given document will expire and be purged.
        Documents don't normally expire; you have to call `set_document_expiration`
        to set a document's expiration time. */
    pub fn document_expiration(&self, doc_id: &str) -> Result<Option<Timestamp>> {
        unsafe {
            let mut error = CBLError::default();
            let exp = CBLDatabase_GetDocumentExpiration(self._ref, as_slice(doc_id), &mut error);
            if exp < 0 {
                return failure(error);
            } else if exp == 0 {
                return Ok(None);
            } else {
                return Ok(Some(Timestamp(exp)));
            }
        }
    }

    /** Sets or clears the expiration time of a document. */
    pub fn set_document_expiration(&mut self, doc_id: &str, when: Option<Timestamp>) -> Result<()> {
        let exp :i64 = match when {
            Some(Timestamp(n)) => n,
            _ => 0,
        };
        unsafe {
            return check_bool(|error| CBLDatabase_SetDocumentExpiration(self._ref, as_slice(doc_id), exp, error));
        }
    }

    /** Registers a document change listener callback. It will be called after a specific document
        is changed on disk. */
    pub fn add_document_change_listener(&self, document: &Document, listener: ChangeListener) -> ListenerToken {
        unsafe {
            let callback: *mut ::std::os::raw::c_void = std::mem::transmute(listener);

            ListenerToken {
                _ref: CBLDatabase_AddDocumentChangeListener(self._ref, CBLDocument_ID(document._ref), Some(c_document_change_listener), callback)
            }
        }
    }

}


//////// DOCUMENT API:


impl Document {

    /** Creates a new, empty document in memory, with an automatically generated unique ID.
        It will not be added to a database until saved. */
    pub fn new() -> Self {
        unsafe { Document{_ref: CBLDocument_Create(), has_ownership: true} }
    }

    /** Creates a new, empty document in memory, with the given ID.
        It will not be added to a database until saved. */
    pub fn new_with_id(id: &str) -> Self {
        unsafe { Document{_ref: CBLDocument_CreateWithID(as_slice(id)), has_ownership: true} }
    }

    /** Returns the document's ID. */
    pub fn id(&self) -> &str {
        unsafe { CBLDocument_ID(self._ref).as_str().unwrap() }
    }

    /** Returns a document's revision ID, which is a short opaque string that's guaranteed to be
        unique to every change made to the document.
        If the document doesn't exist yet, this method returns None. */
    pub fn revision_id(&self) -> Option<&str> {
        unsafe {
            CBLDocument_RevisionID(self._ref).as_str()
        }
    }

    /** Returns a document's current sequence in the local database.
        This number increases every time the document is saved, and a more recently saved document
        will have a greater sequence number than one saved earlier, so sequences may be used as an
        abstract 'clock' to tell relative modification times. */
    pub fn sequence(&self) -> u64 {
        unsafe { CBLDocument_Sequence(self._ref) }
    }

    /** Returns a document's properties as a dictionary.
        This dictionary cannot be mutated; call `mutable_properties` if you want to make
        changes to the document's properties. */
    pub fn properties<'a>(&'a self) -> Dict {
        unsafe { Dict::wrap(CBLDocument_Properties(self._ref), self) }
    }

    /** Returns a document's properties as an mutable dictionary. Any changes made to this
        dictionary will be saved to the database when this Document instance is saved. */
    pub fn mutable_properties(&mut self) -> MutableDict {
        unsafe { MutableDict::adopt(CBLDocument_MutableProperties(self._ref)) }
    }

    /** Replaces a document's properties with the contents of the dictionary.
        The dictionary is retained, not copied, so further changes _will_ affect the document. */
    pub fn set_properties(&mut self, properties: MutableDict) {
        unsafe { CBLDocument_SetProperties(self._ref, properties._ref) }
    }

    /** Returns a document's properties as a JSON string. */
    pub fn properties_as_json(&self) -> String {
        unsafe { CBLDocument_CreateJSON(self._ref).to_string().unwrap() }
    }

    /** Sets a mutable document's properties from a JSON string. */
    pub fn set_properties_as_json(&mut self, json: &str) -> Result<()> {
        unsafe {
            let mut err = CBLError::default();
            let ok = CBLDocument_SetJSON(self._ref, as_slice(json), &mut err);
            return check_failure(ok, &err);
        }
    }
}


impl Drop for Document {
    fn drop(&mut self) {
        if self.has_ownership {
            unsafe {
                release(self._ref)
            }
        }
    }
}


impl Clone for Document {
    fn clone(&self) -> Self {
        unsafe { Document{_ref: retain(self._ref), has_ownership: true} }
    }
}
