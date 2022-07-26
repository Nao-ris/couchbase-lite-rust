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

use super::c_api::*;
use super::slice::*;
use super::*;

/** An in-memory copy of a document. */
#[derive(Debug)]
pub struct Document {
    _ref: *mut CBLDocument,
}

/** Conflict-handling options when saving or deleting a document. */
pub enum ConcurrencyControl {
    LastWriteWins = kCBLConcurrencyControlLastWriteWins as isize,
    FailOnConflict = kCBLConcurrencyControlFailOnConflict as isize,
}

/** Custom conflict handler for use when saving or deleting a document. This handler is called
if the save would cause a conflict, i.e. if the document in the database has been updated
(probably by a pull replicator, or by application code on another thread)
since it was loaded into the CBLDocument being saved. */
type ConflictHandler = fn(&mut Document, &Document) -> bool;
#[no_mangle]
unsafe extern "C" fn c_conflict_handler(
    context: *mut ::std::os::raw::c_void,
    document_being_saved: *mut CBLDocument,
    conflicting_document: *const CBLDocument,
) -> bool {
    let callback: ConflictHandler = std::mem::transmute(context);

    callback(
        &mut Document::retain(document_being_saved),
        &Document::retain(conflicting_document as *mut CBLDocument),
    )
}

/**  A document change listener lets you detect changes made to a specific document after they
are persisted to the database. */
type ChangeListener = fn(&Database, Option<String>);
#[no_mangle]
unsafe extern "C" fn c_document_change_listener(
    context: *mut ::std::os::raw::c_void,
    db: *const CBLDatabase,
    c_doc_id: FLString,
) {
    let callback: ChangeListener = std::mem::transmute(context);

    let database = Database::retain(db as *mut CBLDatabase);

    callback(&database, c_doc_id.to_string());
}

//////// DATABASE'S DOCUMENT API:

impl Database {
    /** Reads a document from the database. Each call to this function returns a new object
    containing the document's current state. */
    pub fn get_document(&self, id: &str) -> Result<Document> {
        unsafe {
            // we always get a mutable CBLDocument,
            // since Rust doesn't let us have MutableDocument subclass.
            let mut error = CBLError::default();
            let doc = CBLDatabase_GetMutableDocument(self.get_ref(), as_slice(id)._ref, &mut error);
            if doc.is_null() {
                if error.code != 0 {
                    return failure(error);
                } else {
                    return Err(Error::cbl_error(CouchbaseLiteError::NotFound));
                }
            }
            return Ok(Document::wrap(doc));
        }
    }

    /** Saves a new or modified document to the database.
    If a newer revision has been saved since \p doc was loaded, it will be overwritten by
    this one. This can lead to data loss! To avoid this, call
    `save_document_with_concurency_control` or
    `save_document_resolving` instead. */
    pub fn save_document(&mut self, doc: &mut Document) -> Result<()> {
        unsafe {
            return check_bool(|error| CBLDatabase_SaveDocument(self.get_ref(), doc._ref, error));
        }
    }

    /** Saves a new or modified document to the database.
    If a conflicting revision has been saved since `doc` was loaded, the `concurrency`
    parameter specifies whether the save should fail, or the conflicting revision should
    be overwritten with the revision being saved.
    If you need finer-grained control, call `save_document_resolving` instead. */
    pub fn save_document_with_concurency_control(
        &mut self,
        doc: &mut Document,
        concurrency: ConcurrencyControl,
    ) -> Result<()> {
        let c_concurrency = concurrency as u8;
        unsafe {
            return check_bool(|error| {
                CBLDatabase_SaveDocumentWithConcurrencyControl(
                    self.get_ref(),
                    doc._ref,
                    c_concurrency,
                    error,
                )
            });
        }
    }

    /** Saves a new or modified document to the database. This function is the same as
    `save_document`, except that it allows for custom conflict handling in the event
    that the document has been updated since `doc` was loaded. */
    pub fn save_document_resolving(
        &mut self,
        doc: &mut Document,
        conflict_handler: ConflictHandler,
    ) -> Result<Document> {
        unsafe {
            let callback: *mut ::std::os::raw::c_void = std::mem::transmute(conflict_handler);
            match check_bool(|error| {
                CBLDatabase_SaveDocumentWithConflictHandler(
                    self.get_ref(),
                    doc._ref,
                    Some(c_conflict_handler),
                    callback,
                    error,
                )
            }) {
                Ok(_) => Ok(doc.to_owned()),
                Err(err) => Err(err),
            }
        }
    }

    /** Deletes a document from the database. Deletions are replicated. */
    pub fn delete_document(&mut self, doc: &Document) -> Result<()> {
        unsafe {
            return check_bool(|error| CBLDatabase_DeleteDocument(self.get_ref(), doc._ref, error));
        }
    }

    /** Deletes a document from the database. Deletions are replicated. */
    pub fn delete_document_with_concurency_control(
        &mut self,
        doc: &Document,
        concurrency: ConcurrencyControl,
    ) -> Result<()> {
        let c_concurrency = concurrency as u8;
        unsafe {
            return check_bool(|error| {
                CBLDatabase_DeleteDocumentWithConcurrencyControl(
                    self.get_ref(),
                    doc._ref,
                    c_concurrency,
                    error,
                )
            });
        }
    }

    /** Purges a document. This removes all traces of the document from the database.
    Purges are _not_ replicated. If the document is changed on a server, it will be re-created */
    pub fn purge_document(&mut self, doc: &Document) -> Result<()> {
        unsafe {
            return check_bool(|error| CBLDatabase_PurgeDocument(self.get_ref(), doc._ref, error));
        }
    }

    /** Purges a document, given only its ID. */
    pub fn purge_document_by_id(&mut self, id: &str) -> Result<()> {
        unsafe {
            return check_bool(|error| {
                CBLDatabase_PurgeDocumentByID(self.get_ref(), as_slice(id)._ref, error)
            });
        }
    }

    /** Returns the time, if any, at which a given document will expire and be purged.
    Documents don't normally expire; you have to call `set_document_expiration`
    to set a document's expiration time. */
    pub fn document_expiration(&self, doc_id: &str) -> Result<Option<Timestamp>> {
        unsafe {
            let mut error = CBLError::default();
            let exp =
                CBLDatabase_GetDocumentExpiration(self.get_ref(), as_slice(doc_id)._ref, &mut error);
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
        let exp: i64 = match when {
            Some(Timestamp(n)) => n,
            _ => 0,
        };
        unsafe {
            return check_bool(|error| {
                CBLDatabase_SetDocumentExpiration(self.get_ref(), as_slice(doc_id)._ref, exp, error)
            });
        }
    }

    /** Registers a document change listener callback. It will be called after a specific document
    is changed on disk. */
    pub fn add_document_change_listener(
        &self,
        document: &Document,
        listener: ChangeListener,
    ) -> ListenerToken {
        unsafe {
            let callback: *mut ::std::os::raw::c_void = std::mem::transmute(listener);

            ListenerToken {
                _ref: CBLDatabase_AddDocumentChangeListener(
                    self.get_ref(),
                    CBLDocument_ID(document._ref),
                    Some(c_document_change_listener),
                    callback,
                ),
            }
        }
    }
}

//////// DOCUMENT API:

impl Document {
    /** Creates a new, empty document in memory, with an automatically generated unique ID.
    It will not be added to a database until saved. */
    pub fn new() -> Self {
        unsafe { Document::wrap(CBLDocument_Create()) }
    }

    /** Creates a new, empty document in memory, with the given ID.
    It will not be added to a database until saved. */
    pub fn new_with_id(id: &str) -> Self {
        unsafe { Document::wrap(CBLDocument_CreateWithID(as_slice(id)._ref)) }
    }

    /** Wrap a CBLDocument as a Document.
    Increment the reference-count for the CBLDocument. */
    pub(crate) fn retain(_ref: *mut CBLDocument) -> Self {
        unsafe { Document { _ref: retain(_ref) } }
    }

    /** Wrap a CBLDocument as a Document.
    The CBLDocument reference-count should already have been incremented from a type-safe source. */
    pub(crate) fn wrap(_ref: *mut CBLDocument) -> Self {
        Document { _ref: _ref }
    }

    pub(crate) fn get_ref(&self) -> *mut CBLDocument {
        self._ref
    }

    /** Returns the document's ID. */
    pub fn id(&self) -> &str {
        unsafe { CBLDocument_ID(self._ref).as_str().unwrap() }
    }

    /** Returns a document's revision ID, which is a short opaque string that's guaranteed to be
    unique to every change made to the document.
    If the document doesn't exist yet, this method returns None. */
    pub fn revision_id(&self) -> Option<&str> {
        unsafe { CBLDocument_RevisionID(self._ref).as_str() }
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
            let ok = CBLDocument_SetJSON(self._ref, as_slice(json)._ref, &mut err);
            return check_failure(ok, &err);
        }
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        unsafe { release(self._ref) }
    }
}

impl Clone for Document {
    fn clone(&self) -> Self {
        Document::retain(self._ref)
    }
}
