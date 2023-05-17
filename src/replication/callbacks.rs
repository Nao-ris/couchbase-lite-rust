use std::ptr;
use crate::{
    CblRef, error, Error, ErrorCode, Dict, Document, CouchbaseLiteError,
    c_api::{
        FLSliceResult, FLSlice_Copy, FLSliceResult_New, CBLError, FLString, FLDict, FLStringResult,
        FLSlice, CBLDocument, CBLDocumentFlags, kCBLDocumentFlagsDeleted,
        kCBLDocumentFlagsAccessRemoved,
    },
    slice::from_bytes,
};

/** Flags describing a replicated document. */
pub static DELETED: u32 = kCBLDocumentFlagsDeleted;
pub static ACCESS_REMOVED: u32 = kCBLDocumentFlagsAccessRemoved;

#[derive(Default)]
pub struct ReplicationConfigurationContext {
    pub push_filter: Option<ReplicationFilter>,
    pub pull_filter: Option<ReplicationFilter>,
    pub conflict_resolver: Option<ConflictResolver>,
    pub property_encryptor: Option<PropertyEncryptor>,
    pub property_decryptor: Option<PropertyDecryptor>,
}

/** A callback that can decide whether a particular document should be pushed or pulled. */
pub type ReplicationFilter = Box<dyn Fn(&Document, bool, bool) -> bool>;

#[no_mangle]
pub(crate) unsafe extern "C" fn c_replication_push_filter(
    context: *mut ::std::os::raw::c_void,
    document: *mut CBLDocument,
    flags: CBLDocumentFlags,
) -> bool {
    let repl_conf_context = context as *const ReplicationConfigurationContext;
    let document = Document::retain(document.cast::<CBLDocument>());
    let (is_deleted, is_access_removed) = read_document_flags(flags);

    (*repl_conf_context)
        .push_filter
        .as_ref()
        .map_or(false, |callback| {
            callback(&document, is_deleted, is_access_removed)
        })
}
pub(crate) unsafe extern "C" fn c_replication_pull_filter(
    context: *mut ::std::os::raw::c_void,
    document: *mut CBLDocument,
    flags: CBLDocumentFlags,
) -> bool {
    let repl_conf_context = context as *const ReplicationConfigurationContext;
    let document = Document::retain(document.cast::<CBLDocument>());
    let (is_deleted, is_access_removed) = read_document_flags(flags);

    (*repl_conf_context)
        .pull_filter
        .as_ref()
        .map_or(false, |callback| {
            callback(&document, is_deleted, is_access_removed)
        })
}
fn read_document_flags(flags: CBLDocumentFlags) -> (bool, bool) {
    (flags & DELETED != 0, flags & ACCESS_REMOVED != 0)
}

/** Conflict-resolution callback for use in replications. This callback will be invoked
when the replicator finds a newer server-side revision of a document that also has local
changes. The local and remote changes must be resolved before the document can be pushed
to the server. */
pub type ConflictResolver =
    Box<dyn Fn(&str, Option<Document>, Option<Document>) -> Option<Document>>;

pub(crate) unsafe extern "C" fn c_replication_conflict_resolver(
    context: *mut ::std::os::raw::c_void,
    document_id: FLString,
    local_document: *const CBLDocument,
    remote_document: *const CBLDocument,
) -> *const CBLDocument {
    let repl_conf_context = context as *const ReplicationConfigurationContext;

    let doc_id = document_id.to_string().unwrap_or_default();
    let local_document = if local_document.is_null() {
        None
    } else {
        Some(Document::retain(local_document as *mut CBLDocument))
    };
    let remote_document = if remote_document.is_null() {
        None
    } else {
        Some(Document::retain(remote_document as *mut CBLDocument))
    };

    (*repl_conf_context)
        .conflict_resolver
        .as_ref()
        .map_or(ptr::null(), |callback| {
            callback(&doc_id, local_document, remote_document)
                .map_or(ptr::null(), |d| d.get_ref() as *const CBLDocument)
        })
}

#[derive(Debug, PartialEq)]
pub enum EncryptionError {
    Temporary, // The replicator will stop the replication when encountering this error, then restart and try encrypting/decrypting the document again
    Permanent, // The replicator will bypass the document and not try encrypting/decrypting the document until a new revision is created
}

/** Callback that encrypts encryptable properties in documents pushed by the replicator.
\note   If a null result or an error is returned, the document will be failed to
        replicate with the kCBLErrorCrypto error. For security reason, the encryption
        cannot be skipped. */
pub type PropertyEncryptor = fn(
    document_id: Option<String>,
    properties: Dict,
    key_path: Option<String>,
    input: Vec<u8>,
    algorithm: Option<String>,
    kid: Option<String>,
    error: &Error,
) -> std::result::Result<Vec<u8>, EncryptionError>;
#[no_mangle]
pub(crate) extern "C" fn c_property_encryptor(
    context: *mut ::std::os::raw::c_void,
    document_id: FLString,
    properties: FLDict,
    key_path: FLString,
    input: FLSlice,
    algorithm: *mut FLStringResult,
    kid: *mut FLStringResult,
    cbl_error: *mut CBLError,
) -> FLSliceResult {
    unsafe {
        let repl_conf_context = context as *const ReplicationConfigurationContext;
        let mut error = cbl_error.as_ref().map_or(Error::default(), Error::new);

        let mut result = FLSliceResult_New(0);
        if let Some(input) = input.to_vec() {
            result = (*repl_conf_context)
                .property_encryptor
                .map(|callback| {
                    callback(
                        document_id.to_string(),
                        Dict::wrap(properties, &properties),
                        key_path.to_string(),
                        input,
                        algorithm.as_ref().and_then(|s| s.clone().to_string()),
                        kid.as_ref().and_then(|s| s.clone().to_string()),
                        &error,
                    )
                })
                .map_or(FLSliceResult_New(0), |v| match v {
                    Ok(v) => FLSlice_Copy(from_bytes(&v[..]).get_ref()),
                    Err(err) => {
                        match err {
                            EncryptionError::Temporary => {
                                error!("Encryption callback returned with transient error");
                                error = Error {
                                    code: ErrorCode::WebSocket(503),
                                    internal_info: None,
                                };
                            }
                            EncryptionError::Permanent => {
                                error!("Encryption callback returned with non transient error");
                                error = Error::cbl_error(CouchbaseLiteError::Crypto);
                            }
                        }

                        FLSliceResult::null()
                    }
                });
        } else {
            error!("Encryption input is None");
            error = Error::cbl_error(CouchbaseLiteError::Crypto);
        }

        if error != Error::default() {
            *cbl_error = error.as_cbl_error();
        }
        result
    }
}

/** Callback that decrypts encrypted encryptable properties in documents pulled by the replicator.
\note   The decryption will be skipped (the encrypted data will be kept) when a null result
        without an error is returned. If an error is returned, the document will be failed to replicate
        with the kCBLErrorCrypto error. */
pub type PropertyDecryptor = fn(
    document_id: Option<String>,
    properties: Dict,
    key_path: Option<String>,
    input: Vec<u8>,
    algorithm: Option<String>,
    kid: Option<String>,
    error: &Error,
) -> std::result::Result<Vec<u8>, EncryptionError>;
#[no_mangle]
pub(crate) extern "C" fn c_property_decryptor(
    context: *mut ::std::os::raw::c_void,
    document_id: FLString,
    properties: FLDict,
    key_path: FLString,
    input: FLSlice,
    algorithm: FLString,
    kid: FLString,
    cbl_error: *mut CBLError,
) -> FLSliceResult {
    unsafe {
        let repl_conf_context = context as *const ReplicationConfigurationContext;
        let mut error = cbl_error.as_ref().map_or(Error::default(), Error::new);

        let mut result = FLSliceResult_New(0);
        if let Some(input) = input.to_vec() {
            result = (*repl_conf_context)
                .property_decryptor
                .map(|callback| {
                    callback(
                        document_id.to_string(),
                        Dict::wrap(properties, &properties),
                        key_path.to_string(),
                        input.to_vec(),
                        algorithm.to_string(),
                        kid.to_string(),
                        &error,
                    )
                })
                .map_or(FLSliceResult_New(0), |v| match v {
                    Ok(v) => FLSlice_Copy(from_bytes(&v[..]).get_ref()),
                    Err(err) => {
                        match err {
                            EncryptionError::Temporary => {
                                error!("Decryption callback returned with transient error");
                                error = Error {
                                    code: ErrorCode::WebSocket(503),
                                    internal_info: None,
                                };
                            }
                            EncryptionError::Permanent => {
                                error!("Decryption callback returned with non transient error");
                                error = Error::cbl_error(CouchbaseLiteError::Crypto);
                            }
                        }

                        FLSliceResult::null()
                    }
                });
        } else {
            error!("Decryption input is None");
            error = Error::cbl_error(CouchbaseLiteError::Crypto);
        }

        if error != Error::default() {
            *cbl_error = error.as_cbl_error();
        }
        result
    }
}
