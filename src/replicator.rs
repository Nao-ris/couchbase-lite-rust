// Couchbase Lite replicator API
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

#![allow(non_upper_case_globals)]

use slice::as_slice;

use std::collections::HashSet;
use std::ptr;

use super::*;
use super::c_api::*;

// WARNING: THIS API IS UNIMPLEMENTED SO FAR


//======== CONFIGURATION


/** Represents the location of a database to replicate with. */
#[derive(Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub(crate) _ref: *mut CBLEndpoint,
}

impl Endpoint {
    pub fn new_with_url(url: String) -> Result<Self> {
        unsafe {
            let mut error = CBLError::default();
            let endpoint: *mut CBLEndpoint = CBLEndpoint_CreateWithURL(as_slice(&url), &mut error as *mut CBLError);

            check_error(&error).and_then(|()| {
                Ok(Self { _ref: retain(endpoint) })
            })
        }
    }

    pub fn new_with_local_db(db: &Database) -> Self {
        unsafe {
            Self { _ref: retain(CBLEndpoint_CreateWithLocalDB(db.get_ref())) }
        }
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        unsafe {
            release(self._ref);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Authenticator {
    pub(crate) _ref: *mut CBLAuthenticator
}

impl Authenticator {
    pub fn create_password(username: String, password: String) -> Self {
        unsafe {
            Self {
                _ref: retain(CBLAuth_CreatePassword(as_slice(&username), as_slice(&password)))
            }
        }
    }

    pub fn create_session(session_id: String, cookie_name: String) -> Self {
        unsafe {
            Self {
                _ref: retain(CBLAuth_CreateSession(as_slice(&session_id), as_slice(&cookie_name)))
            }
        }
    }
}

impl Drop for Authenticator {
    fn drop(&mut self) {
        unsafe {
            release(self._ref);
        }
    }
}


/** Direction of replication: push, pull, or both. */
#[derive(Debug, PartialEq, Eq)]
pub enum ReplicatorType { PushAndPull, Push, Pull }

impl From<CBLReplicatorType> for ReplicatorType {
    fn from(repl_type: CBLReplicatorType) -> Self {
        match repl_type as u32 {
            kCBLReplicatorTypePushAndPull => ReplicatorType::PushAndPull,
            kCBLReplicatorTypePush => ReplicatorType::Push,
            kCBLReplicatorTypePull => ReplicatorType::Pull,
            _ => unreachable!(),
        }
    }
}
impl From<ReplicatorType> for CBLReplicatorType {
    fn from(repl_type: ReplicatorType) -> Self {
        match repl_type {
            ReplicatorType::PushAndPull => kCBLReplicatorTypePushAndPull as u8,
            ReplicatorType::Push => kCBLReplicatorTypePush as u8,
            ReplicatorType::Pull => kCBLReplicatorTypePull as u8,
        }
    }
}

/** Types of proxy servers, for CBLProxySettings. */
#[derive(Debug, PartialEq, Eq)]
pub enum ProxyType { HTTP, HTTPS }

impl From<CBLProxyType> for ProxyType {
    fn from(proxy_type: CBLProxyType) -> Self {
        match proxy_type as u32 {
            kCBLProxyHTTP => ProxyType::HTTP,
            kCBLProxyHTTPS => ProxyType::HTTPS,
            _ => unreachable!(),
        }
    }
}
impl From<ProxyType> for CBLProxyType {
    fn from(proxy_type: ProxyType) -> Self {
        match proxy_type {
            ProxyType::HTTP => kCBLProxyHTTP as u8,
            ProxyType::HTTPS => kCBLProxyHTTPS as u8,
        }
    }
}

/** Proxy settings for the replicator. */
#[derive(Debug, PartialEq, Eq)]
pub struct ProxySettings {
    pub proxy_type: ProxyType,          // Type of proxy
    pub hostname:   Option<String>,            // Proxy server hostname or IP address
    pub port:       u16,                // Proxy server port
    pub username:   Option<String>,    // Username for proxy auth
    pub password:   Option<String>     // Password for proxy auth
}

impl From<&CBLProxySettings> for ProxySettings {
    fn from(proxy_settings: &CBLProxySettings) -> Self {
        ProxySettings {
            proxy_type: proxy_settings.type_.into(),
            hostname: unsafe { proxy_settings.hostname.to_string() },
            port: proxy_settings.port,
            username: unsafe { proxy_settings.username.to_string() },
            password: unsafe { proxy_settings.password.to_string() },
        }
    }
}
impl From<ProxySettings> for CBLProxySettings {
    fn from(proxy_settings: ProxySettings) -> Self {
        CBLProxySettings {
            type_: proxy_settings.proxy_type.into(),
            hostname: proxy_settings.hostname.map(|s| unsafe { FLSlice_Copy(as_slice(&s)).as_slice() }).unwrap_or(slice::NULL_SLICE),
            port: proxy_settings.port,
            username: proxy_settings.username.map(|s| unsafe { FLSlice_Copy(as_slice(&s)).as_slice() }).unwrap_or(slice::NULL_SLICE),
            password: proxy_settings.password.map(|s| unsafe { FLSlice_Copy(as_slice(&s)).as_slice() }).unwrap_or(slice::NULL_SLICE),
        }
    }
}


/** A callback that can decide whether a particular document should be pushed or pulled. */
pub type ReplicationFilter =  fn(document: &Document, is_deleted: bool, is_access_removed: bool) -> bool;
#[no_mangle]
unsafe extern "C" fn c_replication_push_filter(
    context: *mut ::std::os::raw::c_void,
    document: *mut CBLDocument,
    flags: CBLDocumentFlags,
) -> bool {
    let repl_conf_context: *const ReplicationConfigurationContext = std::mem::transmute(context);

    let document = Document {
        _ref: retain(document as *mut CBLDocument),
    };

    let (is_deleted, is_access_removed) = read_document_flags(flags);

    (*repl_conf_context).push_filter
        .map(|callback| callback(&document, is_deleted, is_access_removed))
        .unwrap_or(false)
}
unsafe extern "C" fn c_replication_pull_filter(
    context: *mut ::std::os::raw::c_void,
    document: *mut CBLDocument,
    flags: CBLDocumentFlags,
) -> bool {
    let repl_conf_context: *const ReplicationConfigurationContext = std::mem::transmute(context);

    let document = Document {
        _ref: retain(document as *mut CBLDocument),
    };

    let (is_deleted, is_access_removed) = read_document_flags(flags);

    (*repl_conf_context).pull_filter
        .map(|callback| callback(&document, is_deleted, is_access_removed))
        .unwrap_or(false)
}
fn read_document_flags(flags: CBLDocumentFlags) -> (bool, bool) {
    (flags & DELETED != 0, flags & ACCESS_REMOVED != 0)
}

/** Conflict-resolution callback for use in replications. This callback will be invoked
    when the replicator finds a newer server-side revision of a document that also has local
    changes. The local and remote changes must be resolved before the document can be pushed
    to the server. */
pub type ConflictResolver = fn(document_id: &str,
                               local_document: Option<Document>,
                               remote_document: Option<Document>) -> Option<Document>;
unsafe extern "C" fn c_replication_conflict_resolver(
    context: *mut ::std::os::raw::c_void,
    document_id: FLString,
    local_document: *const CBLDocument,
    remote_document: *const CBLDocument,
) -> *const CBLDocument {
    let repl_conf_context: *const ReplicationConfigurationContext = std::mem::transmute(context);

    let doc_id = document_id.to_string().unwrap_or("".to_string());
    let local_document = if local_document.is_null() {
        Some(Document {
            _ref: retain(local_document as *mut CBLDocument),
        })
    } else {
        None
    };
    let remote_document = if remote_document.is_null() {
        Some(Document {
            _ref: retain(remote_document as *mut CBLDocument),
        })
    } else {
        None
    };

    if let Some(callback) = (*repl_conf_context).conflict_resolver {
        callback(&doc_id, local_document, remote_document)
            .map(|d| d._ref as *const CBLDocument)
            .unwrap_or(ptr::null())
    } else {
        ptr::null()
    }
}

/** Callback that encrypts encryptable properties in documents pushed by the replicator.
    \note   If a null result or an error is returned, the document will be failed to
            replicate with the kCBLErrorCrypto error. For security reason, the encryption
            cannot be skipped. */
pub type PropertyEncryptor = fn(
    document_id: Option<String>,
    properties: Dict,
    key_path: Option<String>,
    input: Option<String>,
    algorithm: Option<String>,
    kid: Option<String>,
    error: &Error
) -> String;
#[no_mangle]
pub extern "C" fn c_property_encryptor(
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
        let repl_conf_context: *const ReplicationConfigurationContext = std::mem::transmute(context);

        let error = cbl_error.as_ref().map(|e| Error::new(e)).unwrap_or(Error::default());

        let result = (*repl_conf_context).property_encryptor
            .map(|callback| {
                callback(
                    document_id.to_string(),
                    Dict::wrap(properties, &properties),
                    key_path.to_string(),
                    input.to_string(),
                    algorithm.as_ref().and_then(|s| s.to_string()),
                    kid.as_ref().and_then(|s| s.to_string()),
                    &error,
                )
            })
            .map(|s| FLSlice_Copy(as_slice(&s)))
            .unwrap_or(FLSliceResult_New(0));

        if !cbl_error.is_null() {
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
    input: Option<String>,
    algorithm: Option<String>,
    kid: Option<String>,
    error: &Error
) -> String;
#[no_mangle]
pub extern "C" fn c_property_decryptor(
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
        let repl_conf_context: *const ReplicationConfigurationContext = std::mem::transmute(context);

        let error = cbl_error.as_ref().map(|e| Error::new(e)).unwrap_or(Error::default());

        let result = (*repl_conf_context).property_decryptor
            .map(|callback| {
                callback(
                    document_id.to_string(),
                    Dict::wrap(properties, &properties),
                    key_path.to_string(),
                    input.to_string(),
                    algorithm.to_string(),
                    kid.to_string(),
                    &error,
                )
            })
            .map(|s| FLSlice_Copy(as_slice(&s)))
            .unwrap_or(FLSliceResult_New(0));

        if !cbl_error.is_null() {
            *cbl_error = error.as_cbl_error();
        }
        result
    }
}


struct ReplicationConfigurationContext {
    pub push_filter:               Option<ReplicationFilter>,
    pub pull_filter:               Option<ReplicationFilter>,
    pub conflict_resolver:         Option<ConflictResolver>,
    pub property_encryptor:        Option<PropertyEncryptor>,
    pub property_decryptor:        Option<PropertyDecryptor>,
}

/** The configuration of a replicator. */
pub struct ReplicatorConfiguration<'c> {
    pub database:                  Database,                     // The database to replicate
    pub endpoint:                  Endpoint,                     // The address of the other database to replicate with
    pub replicator_type:           ReplicatorType,                   // Push, pull or both
    pub continuous:                bool,                             // Continuous replication?
    //-- Auto Purge:
    /**
    If auto purge is active, then the library will automatically purge any documents that the replicating
    user loses access to via the Sync Function on Sync Gateway.  If disableAutoPurge is true, this behavior
    is disabled and an access removed event will be sent to any document listeners that are active on the
    replicator.

    IMPORTANT: For performance reasons, the document listeners must be added *before* the replicator is started
    or they will not receive the events.
    */
    pub disable_auto_purge:        bool,
    //-- Retry Logic:
    pub max_attempts:              u32,	                             //< Max retry attempts where the initial connect to replicate counts toward the given value.
                                                                     //< Specify 0 to use the default value, 10 times for a non-continuous replicator and max-int time for a continuous replicator. Specify 1 means there will be no retry after the first attempt.
    pub max_attempt_wait_time:     u32,	                             //< Max wait time between retry attempts in seconds. Specify 0 to use the default value of 300 seconds.
    //-- WebSocket:
    pub heartbeat:                 u32,                              //< The heartbeat interval in seconds. Specify 0 to use the default value of 300 seconds.
    pub authenticator:             Option<Authenticator>,                // Authentication credentials, if needed
    pub proxy:                     Option<ProxySettings>,            // HTTP client proxy settings
    pub headers:                   Dict<'c>,                     // Extra HTTP headers to add to the WebSocket request
    //-- TLS settings:
    pub pinned_server_certificate: Option<&'c [u8]>,                  // An X.509 cert to "pin" TLS connections to (PEM or DER)
    pub trusted_root_certificates: Option<&'c [u8]>,                  // Set of anchor certs (PEM format)
    //-- Filtering:
    pub channels:                  Array<'c>,             // Optional set of channels to pull from
    pub document_ids:              Array<'c>,             // Optional set of document IDs to replicate
    pub push_filter:               Option<ReplicationFilter>,        // Optional callback to filter which docs are pushed
    pub pull_filter:               Option<ReplicationFilter>,        // Optional callback to validate incoming docs
    pub conflict_resolver:         Option<ConflictResolver>,         // Optional conflict-resolver callback
    //-- Property Encryption
    pub property_encryptor:        Option<PropertyEncryptor>,	     //< Optional callback to encrypt \ref CBLEncryptable values.
    pub property_decryptor:        Option<PropertyDecryptor>,        //< Optional callback to decrypt encrypted \ref CBLEncryptable values.
}

impl<'c> From<&'c CBLReplicatorConfiguration> for ReplicatorConfiguration<'c> {
    fn from(config: &'c CBLReplicatorConfiguration) -> Self {
        unsafe {
            let context: *const ReplicationConfigurationContext = std::mem::transmute(config.context);

            ReplicatorConfiguration {
                database: Database::new_no_retain(config.database),
                endpoint: Endpoint { _ref: config.endpoint },
                replicator_type: config.replicatorType.into(),
                continuous: config.continuous,
                disable_auto_purge: config.disableAutoPurge,
                max_attempts: config.maxAttempts,
                max_attempt_wait_time: config.maxAttemptWaitTime,
                heartbeat: config.heartbeat,
                authenticator: if config.authenticator.is_null() {
                    None
                } else {
                    Some(Authenticator { _ref: retain(config.authenticator) })
                },
                proxy: config.proxy.as_ref().map(|proxy| proxy.into()),
                headers: Dict::wrap(config.headers, &config.headers),
                pinned_server_certificate: config.pinnedServerCertificate.as_byte_array(),
                trusted_root_certificates: config.trustedRootCertificates.as_byte_array(),
                channels: Array::wrap(config.channels, &config.channels),
                document_ids: Array::wrap(config.documentIDs, &config.documentIDs),
                push_filter: (*context).push_filter,
                pull_filter: (*context).pull_filter,
                conflict_resolver: (*context).conflict_resolver,
                property_encryptor: (*context).property_encryptor,
                property_decryptor: (*context).property_encryptor,
            }
        }
    }
}
impl<'c> From<ReplicatorConfiguration<'c>> for CBLReplicatorConfiguration {
    fn from(config: ReplicatorConfiguration<'c>) -> Self {
        let context: Box<ReplicationConfigurationContext> = Box::new(ReplicationConfigurationContext {
            push_filter: config.push_filter,
            pull_filter: config.pull_filter,
            conflict_resolver: config.conflict_resolver,
            property_encryptor: config.property_encryptor,
            property_decryptor: config.property_decryptor,
        });
        let context = Box::into_raw(context);

        let proxy = config.proxy
            .map(|p| Box::new(p.into()))
            .map(|b| Box::into_raw(b))
            .unwrap_or(ptr::null_mut());
        unsafe {
            CBLReplicatorConfiguration {
                database: retain(config.database.get_ref()),
                endpoint: retain(config.endpoint._ref),
                replicatorType: config.replicator_type.into(),
                continuous: config.continuous,
                disableAutoPurge: config.disable_auto_purge,
                maxAttempts: config.max_attempts,
                maxAttemptWaitTime: config.max_attempt_wait_time,
                heartbeat: config.heartbeat,
                authenticator: config.authenticator.map(|a| a._ref).unwrap_or(ptr::null_mut()),
                proxy: proxy,
                headers: config.headers._ref,
                pinnedServerCertificate: config.pinned_server_certificate.map(|c| slice::bytes_as_slice(c)).unwrap_or(slice::NULL_SLICE),
                trustedRootCertificates: config.trusted_root_certificates.map(|c| slice::bytes_as_slice(c)).unwrap_or(slice::NULL_SLICE),
                channels: config.channels._ref,
                documentIDs: config.document_ids._ref,
                pushFilter: (*context).push_filter.and(Some(c_replication_push_filter)),
                pullFilter: (*context).pull_filter.and(Some(c_replication_pull_filter)),
                conflictResolver: (*context).conflict_resolver.and(Some(c_replication_conflict_resolver)),
                context: std::mem::transmute(context),
                propertyEncryptor: (*context).push_filter.and(Some(c_property_encryptor)),
                propertyDecryptor: (*context).push_filter.and(Some(c_property_decryptor)),
            }
        }
    }
}


//======== LIFECYCLE

/** A background task that syncs a \ref Database with a remote server or peer. */
pub struct Replicator {
    _ref: *mut CBLReplicator
}

impl Replicator {
    /** Creates a replicator with the given configuration. */
    pub fn new(config: ReplicatorConfiguration) -> Result<Replicator> {
        unsafe {
            let cbl_config: Box<CBLReplicatorConfiguration> = Box::new(config.into());
            let cbl_config = Box::into_raw(cbl_config);

            let mut error = CBLError::default();
            let replicator = CBLReplicator_Create(cbl_config, &mut error as *mut CBLError);

            check_error(&error).and_then(|()| {
                Ok(Replicator { _ref: replicator })
            })
        }
    }

    /** Returns the configuration of an existing replicator. */
    pub fn config(&self) -> Option<ReplicatorConfiguration> {
        unsafe {
            let cbl_config = CBLReplicator_Config(self._ref);
            cbl_config.as_ref().map(|c| c.into())
        }
    }

    /** Starts a replicator, asynchronously. Does nothing if it's already started. */
    pub fn start(&mut self, reset_checkpoint: bool) {
        unsafe {
            CBLReplicator_Start(self._ref, reset_checkpoint);
        }
    }

    /** Stops a running replicator, asynchronously. Does nothing if it's not already started.
        The replicator will call your \ref CBLReplicatorChangeListener with an activity level of
        \ref kCBLReplicatorStopped after it stops. Until then, consider it still active. */
    pub fn stop(&mut self) {
        unsafe {
            CBLReplicator_Stop(self._ref);
        }
    }

    /** Informs the replicator whether it's considered possible to reach the remote host with
        the current network configuration. The default value is true. This only affects the
        replicator's behavior while it's in the Offline state:
        * Setting it to false will cancel any pending retry and prevent future automatic retries.
        * Setting it back to true will initiate an immediate retry.*/
    pub fn set_host_reachable(&mut self, reachable: bool) {
        unsafe {
            CBLReplicator_SetHostReachable(self._ref, reachable);
        }
    }

    /** Puts the replicator in or out of "suspended" state. The default is false.
        * Setting suspended=true causes the replicator to disconnect and enter Offline state;
          it will not attempt to reconnect while it's suspended.
        * Setting suspended=false causes the replicator to attempt to reconnect, _if_ it was
          connected when suspended, and is still in Offline state. */
    pub fn set_suspended(&mut self, suspended: bool) {
        unsafe {
            CBLReplicator_SetSuspended(self._ref, suspended);
        }
    }

}

impl Drop for Replicator {
    fn drop(&mut self) {
        unsafe {
            release(self._ref)
        }
    }
}

impl Clone for Replicator {
    fn clone(&self) -> Self {
        unsafe {
            return Replicator {
                _ref: retain(self._ref)
            }
        }
    }
}


//======== STATUS AND PROGRESS


/** The possible states a replicator can be in during its lifecycle. */
#[derive(Debug)]
pub enum ReplicatorActivityLevel {
    Stopped,            // The replicator is unstarted, finished, or hit a fatal error.
    Offline,            // The replicator is offline, as the remote host is unreachable.
    Connecting,         // The replicator is connecting to the remote host.
    Idle,               // The replicator is inactive, waiting for changes to sync.
    Busy                // The replicator is actively transferring data.
}

impl From<u8> for ReplicatorActivityLevel {
    fn from(level: u8) -> Self {
        match level as u32 {
            kCBLReplicatorStopped => ReplicatorActivityLevel::Stopped,
            kCBLReplicatorOffline => ReplicatorActivityLevel::Offline,
            kCBLReplicatorConnecting => ReplicatorActivityLevel::Connecting,
            kCBLReplicatorIdle => ReplicatorActivityLevel::Idle,
            kCBLReplicatorBusy => ReplicatorActivityLevel::Busy,
            _ => unreachable!(),
        }
    }
}

/** The current progress status of a Replicator. The `fraction_complete` ranges from 0.0 to 1.0 as
    replication progresses. The value is very approximate and may bounce around during replication;
    making it more accurate would require slowing down the replicator and incurring more load on the
    server. It's fine to use in a progress bar, though. */
pub struct ReplicatorProgress {
    pub fraction_complete: f32,     // Very-approximate completion, from 0.0 to 1.0
    pub document_count:    u64      // Number of documents transferred so far
}

/** A replicator's current status. */
pub struct ReplicatorStatus {
    pub activity: ReplicatorActivityLevel,  // Current state
    pub progress: ReplicatorProgress,       // Approximate fraction complete
    pub error:    Result<()>                // Error, if any
}

impl From<CBLReplicatorStatus> for ReplicatorStatus {
    fn from(status: CBLReplicatorStatus) -> Self {
        ReplicatorStatus {
            activity: status.activity.into(),
            progress: ReplicatorProgress {
                fraction_complete: status.progress.complete,
                document_count: status.progress.documentCount,
            },
            error: check_error(&status.error),
        }
    }
}

/** A callback that notifies you when the replicator's status changes. */
pub type ReplicatorChangeListener = fn(&Replicator, ReplicatorStatus);
#[no_mangle]
unsafe extern "C" fn c_replicator_change_listener(
    context: *mut ::std::os::raw::c_void,
    replicator: *mut CBLReplicator,
    status: *const CBLReplicatorStatus,
) {
    let callback: ReplicatorChangeListener = std::mem::transmute(context);

    let replicator = Replicator { _ref: retain(replicator) };
    let status: ReplicatorStatus = (*status).into();

    callback(&replicator, status);
}

/** A callback that notifies you when documents are replicated. */
pub type ReplicatedDocumentListener = fn(&Replicator, Direction, Vec<ReplicatedDocument>);
unsafe extern "C" fn c_replicator_document_change_listener(
    context: *mut ::std::os::raw::c_void,
    replicator: *mut CBLReplicator,
    is_push: bool,
    num_documents: u32,
    documents: *const CBLReplicatedDocument,
) {
    let callback: ReplicatedDocumentListener = std::mem::transmute(context);

    let replicator = Replicator { _ref: retain(replicator) };
    let direction = if is_push { Direction::Pushed } else { Direction::Pulled};

    let repl_documents = std::slice::from_raw_parts(documents, num_documents as usize)
        .iter()
        .filter_map(|document| {
            if let Some(doc_id) = document.ID.to_string() {
                Some(ReplicatedDocument {
                    id: doc_id,
                    flags: document.flags,
                    error: check_error(&document.error),
                })
            } else {
                None
            }
        })
        .collect();

    callback(&replicator, direction, repl_documents);
}

/** Flags describing a replicated document. */
pub static DELETED        : u32 = kCBLDocumentFlagsDeleted;
pub static ACCESS_REMOVED : u32 = kCBLDocumentFlagsAccessRemoved;

/** Information about a document that's been pushed or pulled. */
pub struct ReplicatedDocument {
    pub id:     String,                    // The document ID
    pub flags:  u32,                        // Indicates whether the document was deleted or removed
    pub error:  Result<()>                  // Error, if document failed to replicate
}

/** Direction of document transfer. */
#[derive(Debug)]
pub enum Direction {Pulled, Pushed }

impl Replicator {

    /** Returns the replicator's current status. */
    pub fn status(&self) -> ReplicatorStatus {
        unsafe {
            CBLReplicator_Status(self._ref).into()
        }
    }

    /** Indicates which documents have local changes that have not yet been pushed to the server
        by this replicator. This is of course a snapshot, that will go out of date as the replicator
        makes progress and/or documents are saved locally. */
    pub fn pending_document_ids(&self) -> Result<HashSet<String>> {
        unsafe {
            let mut error = CBLError::default();
            let docs: FLDict = CBLReplicator_PendingDocumentIDs(self._ref, &mut error as *mut CBLError);

            check_error(&error).and_then(|()| {
                if docs.is_null() {
                    return Err(Error::default());
                }

                let dict = Dict::wrap(docs, self);
                Ok(dict.to_keys_hash_set())
            })
        }
    }

    /** Indicates whether the document with the given ID has local changes that have not yet been
        pushed to the server by this replicator.

        This is equivalent to, but faster than, calling \ref pending_document_ids and
        checking whether the result contains \p docID. See that function's documentation for details. */
    pub fn is_document_pending(&self, doc_id: &str) -> Result<bool> {
        unsafe {
            let mut error = CBLError::default();
            let result = CBLReplicator_IsDocumentPending(self._ref, as_slice(doc_id), &mut error as *mut CBLError);

            check_error(&error).and_then(|()| {
                Ok(result)
            })
        }
    }

    /** Adds a listener that will be called when the replicator's status changes. */
    pub fn add_change_listener(&mut self, listener: ReplicatorChangeListener) -> ListenerToken {
        unsafe {
            let callback: *mut ::std::os::raw::c_void = std::mem::transmute(listener);

            ListenerToken {
                _ref: CBLReplicator_AddChangeListener(self._ref, Some(c_replicator_change_listener), callback)
            }
        }
    }

    /** Adds a listener that will be called when documents are replicated. */
    pub fn add_document_listener(&mut self, listener: ReplicatedDocumentListener) -> ListenerToken {
        unsafe {
            let callback: *mut ::std::os::raw::c_void = std::mem::transmute(listener);

            ListenerToken {
                _ref: CBLReplicator_AddDocumentReplicationListener(self._ref, Some(c_replicator_document_change_listener), callback)
            }
        }
    }
}
