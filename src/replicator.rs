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

use std::{
    ptr,
    collections::{HashMap, HashSet},
    sync::mpsc::channel,
    time::Duration,
};
use crate::{
    CblRef, CouchbaseLiteError, Database, Dict, Document, Error, ErrorCode, ListenerToken,
    MutableDict, Result, check_error, release, retain,
    slice::{from_str, from_bytes, self},
    c_api::{
        CBLListener_Remove, CBLAuth_CreatePassword, CBLAuth_CreateSession, CBLAuthenticator,
        CBLDocument, CBLDocumentFlags, CBLEndpoint, CBLEndpoint_CreateWithLocalDB,
        CBLEndpoint_CreateWithURL, CBLError, CBLProxySettings, CBLProxyType, CBLReplicatedDocument,
        CBLReplicator, CBLReplicatorConfiguration, CBLReplicatorStatus, CBLReplicatorType,
        CBLReplicator_AddChangeListener, CBLReplicator_AddDocumentReplicationListener,
        CBLReplicator_Create, CBLReplicator_IsDocumentPending, CBLReplicator_PendingDocumentIDs,
        CBLReplicator_SetHostReachable, CBLReplicator_SetSuspended, CBLReplicator_Start,
        CBLReplicator_Status, CBLReplicator_Stop, FLDict, FLSlice, FLSliceResult,
        FLSliceResult_New, FLSlice_Copy, FLString, FLStringResult, kCBLDocumentFlagsAccessRemoved,
        kCBLDocumentFlagsDeleted, kCBLProxyHTTP, kCBLProxyHTTPS, kCBLReplicatorBusy,
        kCBLReplicatorConnecting, kCBLReplicatorIdle, kCBLReplicatorOffline, kCBLReplicatorStopped,
        kCBLReplicatorTypePull, kCBLReplicatorTypePush, kCBLReplicatorTypePushAndPull,
    },
    MutableArray, Listener, error,
};

// WARNING: THIS API IS UNIMPLEMENTED SO FAR

//======== CONFIGURATION

/** Represents the location of a database to replicate with. */
#[derive(Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub(crate) cbl_ref: *mut CBLEndpoint,
    pub url: Option<String>,
}

impl CblRef for Endpoint {
    type Output = *mut CBLEndpoint;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Endpoint {
    pub fn new_with_url(url: &str) -> Result<Self> {
        unsafe {
            let mut error = CBLError::default();
            let endpoint: *mut CBLEndpoint =
                CBLEndpoint_CreateWithURL(from_str(url).get_ref(), std::ptr::addr_of_mut!(error));

            check_error(&error).map(|_| Self {
                cbl_ref: endpoint,
                url: Some(url.to_string()),
            })
        }
    }

    pub fn new_with_local_db(db: &Database) -> Self {
        unsafe {
            Self {
                cbl_ref: CBLEndpoint_CreateWithLocalDB(db.get_ref()),
                url: None,
            }
        }
    }
}

impl Clone for Endpoint {
    fn clone(&self) -> Self {
        Self {
            cbl_ref: self.cbl_ref,
            url: self.url.clone(),
        }
    }
}

/** An opaque object representing authentication credentials for a remote server. */
#[derive(Debug, PartialEq, Eq)]
pub struct Authenticator {
    pub(crate) cbl_ref: *mut CBLAuthenticator,
}

impl CblRef for Authenticator {
    type Output = *mut CBLAuthenticator;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Authenticator {
    pub fn create_password(username: &str, password: &str) -> Self {
        unsafe {
            Self {
                cbl_ref: CBLAuth_CreatePassword(
                    from_str(username).get_ref(),
                    from_str(password).get_ref(),
                ),
            }
        }
    }

    pub fn create_session(session_id: &str, cookie_name: &str) -> Self {
        unsafe {
            Self {
                cbl_ref: CBLAuth_CreateSession(
                    from_str(session_id).get_ref(),
                    from_str(cookie_name).get_ref(),
                ),
            }
        }
    }
}

impl Clone for Authenticator {
    fn clone(&self) -> Self {
        Self {
            cbl_ref: self.cbl_ref,
        }
    }
}

/** Direction of replication: push, pull, or both. */
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicatorType {
    PushAndPull,
    Push,
    Pull,
}

impl From<CBLReplicatorType> for ReplicatorType {
    fn from(repl_type: CBLReplicatorType) -> Self {
        match u32::from(repl_type) {
            kCBLReplicatorTypePushAndPull => Self::PushAndPull,
            kCBLReplicatorTypePush => Self::Push,
            kCBLReplicatorTypePull => Self::Pull,
            _ => unreachable!(),
        }
    }
}
impl From<ReplicatorType> for CBLReplicatorType {
    fn from(repl_type: ReplicatorType) -> Self {
        match repl_type {
            ReplicatorType::PushAndPull => kCBLReplicatorTypePushAndPull as Self,
            ReplicatorType::Push => kCBLReplicatorTypePush as Self,
            ReplicatorType::Pull => kCBLReplicatorTypePull as Self,
        }
    }
}

/** Types of proxy servers, for CBLProxySettings. */
#[derive(Debug, PartialEq, Eq)]
pub enum ProxyType {
    HTTP,
    HTTPS,
}

impl From<CBLProxyType> for ProxyType {
    fn from(proxy_type: CBLProxyType) -> Self {
        match u32::from(proxy_type) {
            kCBLProxyHTTP => Self::HTTP,
            kCBLProxyHTTPS => Self::HTTPS,
            _ => unreachable!(),
        }
    }
}
impl From<ProxyType> for CBLProxyType {
    fn from(proxy_type: ProxyType) -> Self {
        match proxy_type {
            ProxyType::HTTP => kCBLProxyHTTP as Self,
            ProxyType::HTTPS => kCBLProxyHTTPS as Self,
        }
    }
}

/** Proxy settings for the replicator. */
#[derive(Debug)]
pub struct ProxySettings {
    pub hostname: Option<String>, // Proxy server hostname or IP address
    pub username: Option<String>, // Username for proxy auth
    pub password: Option<String>, // Password for proxy auth
    cbl: CBLProxySettings,
}

impl ProxySettings {
    pub fn new(
        proxy_type: ProxyType,
        hostname: Option<String>,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let cbl = CBLProxySettings {
            type_: proxy_type.into(),
            hostname: hostname
                .as_ref()
                .map_or(slice::NULL_SLICE, |s| from_str(s).get_ref()),
            port,
            username: username
                .as_ref()
                .map_or(slice::NULL_SLICE, |s| from_str(s).get_ref()),
            password: password
                .as_ref()
                .map_or(slice::NULL_SLICE, |s| from_str(s).get_ref()),
        };

        Self {
            hostname,
            username,
            password,
            cbl,
        }
    }
}

impl CblRef for ProxySettings {
    type Output = *const CBLProxySettings;
    fn get_ref(&self) -> Self::Output {
        std::ptr::addr_of!(self.cbl)
    }
}

/** A callback that can decide whether a particular document should be pushed or pulled. */
pub type ReplicationFilter = Box<dyn Fn(&Document, bool, bool) -> bool>;

#[no_mangle]
unsafe extern "C" fn c_replication_push_filter(
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
unsafe extern "C" fn c_replication_pull_filter(
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

unsafe extern "C" fn c_replication_conflict_resolver(
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

#[derive(Default)]
pub struct ReplicationConfigurationContext {
    pub push_filter: Option<ReplicationFilter>,
    pub pull_filter: Option<ReplicationFilter>,
    pub conflict_resolver: Option<ConflictResolver>,
    pub property_encryptor: Option<PropertyEncryptor>,
    pub property_decryptor: Option<PropertyDecryptor>,
}

/** The configuration of a replicator. */
pub struct ReplicatorConfiguration {
    pub database: Database,              // The database to replicate
    pub endpoint: Endpoint,              // The address of the other database to replicate with
    pub replicator_type: ReplicatorType, // Push, pull or both
    pub continuous: bool,                // Continuous replication?
    //-- Auto Purge:
    /**
    If auto purge is active, then the library will automatically purge any documents that the replicating
    user loses access to via the Sync Function on Sync Gateway.  If disableAutoPurge is true, this behavior
    is disabled and an access removed event will be sent to any document listeners that are active on the
    replicator.

    IMPORTANT: For performance reasons, the document listeners must be added *before* the replicator is started
    or they will not receive the events.
    */
    pub disable_auto_purge: bool,
    //-- Retry Logic:
    pub max_attempts: u32, //< Max retry attempts where the initial connect to replicate counts toward the given value.
    //< Specify 0 to use the default value, 10 times for a non-continuous replicator and max-int time for a continuous replicator. Specify 1 means there will be no retry after the first attempt.
    pub max_attempt_wait_time: u32, //< Max wait time between retry attempts in seconds. Specify 0 to use the default value of 300 seconds.
    //-- WebSocket:
    pub heartbeat: u32, //< The heartbeat interval in seconds. Specify 0 to use the default value of 300 seconds.
    pub authenticator: Option<Authenticator>, // Authentication credentials, if needed
    pub proxy: Option<ProxySettings>, // HTTP client proxy settings
    pub headers: HashMap<String, String>, // Extra HTTP headers to add to the WebSocket request
    //-- TLS settings:
    pub pinned_server_certificate: Option<Vec<u8>>, // An X.509 cert to "pin" TLS connections to (PEM or DER)
    pub trusted_root_certificates: Option<Vec<u8>>, // Set of anchor certs (PEM format)
    //-- Filtering:
    pub channels: MutableArray, // Optional set of channels to pull from
    pub document_ids: MutableArray, // Optional set of document IDs to replicate
    //-- Advanced HTTP settings:
    /** The option to remove the restriction that does not allow the replicator to save the parent-domain
    cookies, the cookies whose domains are the parent domain of the remote host, from the HTTP
    response. For example, when the option is set to true, the cookies whose domain are “.foo.com”
    returned by “bar.foo.com” host will be permitted to save. This is only recommended if the host
    issuing the cookie is well trusted.
    This option is disabled by default (see \ref kCBLDefaultReplicatorAcceptParentCookies) which means
    that the parent-domain cookies are not permitted to save by default. */
    pub accept_parent_domain_cookies: bool,
}

//======== LIFECYCLE

type ReplicatorsListeners<T> = Vec<Listener<Box<T>>>;

/** A background task that syncs a \ref Database with a remote server or peer. */
pub struct Replicator {
    cbl_ref: *mut CBLReplicator,
    pub config: Option<ReplicatorConfiguration>,
    pub headers: Option<MutableDict>,
    pub context: Option<Box<ReplicationConfigurationContext>>,
    change_listeners: ReplicatorsListeners<ReplicatorChangeListener>,
    document_listeners: ReplicatorsListeners<ReplicatedDocumentListener>,
}

impl CblRef for Replicator {
    type Output = *mut CBLReplicator;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Replicator {
    /** Creates a replicator with the given configuration. */
    pub fn new(
        config: ReplicatorConfiguration,
        context: Box<ReplicationConfigurationContext>,
    ) -> Result<Self> {
        unsafe {
            let headers = MutableDict::from_hashmap(&config.headers);

            let cbl_config = CBLReplicatorConfiguration {
                database: retain(config.database.get_ref()),
                endpoint: config.endpoint.get_ref(),
                replicatorType: config.replicator_type.clone().into(),
                continuous: config.continuous,
                disableAutoPurge: config.disable_auto_purge,
                maxAttempts: config.max_attempts,
                maxAttemptWaitTime: config.max_attempt_wait_time,
                heartbeat: config.heartbeat,
                authenticator: config
                    .authenticator
                    .as_ref()
                    .map_or(ptr::null_mut(), CblRef::get_ref),
                proxy: config
                    .proxy
                    .as_ref()
                    .map_or(ptr::null_mut(), CblRef::get_ref),
                headers: headers.as_dict().get_ref(),
                pinnedServerCertificate: config
                    .pinned_server_certificate
                    .as_ref()
                    .map_or(slice::NULL_SLICE, |c| slice::from_bytes(c).get_ref()),
                trustedRootCertificates: config
                    .trusted_root_certificates
                    .as_ref()
                    .map_or(slice::NULL_SLICE, |c| slice::from_bytes(c).get_ref()),
                channels: config.channels.get_ref(),
                documentIDs: config.document_ids.get_ref(),
                pushFilter: context
                    .push_filter
                    .as_ref()
                    .and(Some(c_replication_push_filter)),
                pullFilter: context
                    .pull_filter
                    .as_ref()
                    .and(Some(c_replication_pull_filter)),
                conflictResolver: context
                    .conflict_resolver
                    .as_ref()
                    .and(Some(c_replication_conflict_resolver)),
                propertyEncryptor: context
                    .property_encryptor
                    .as_ref()
                    .and(Some(c_property_encryptor)),
                propertyDecryptor: context
                    .property_decryptor
                    .as_ref()
                    .and(Some(c_property_decryptor)),
                acceptParentDomainCookies: config.accept_parent_domain_cookies,
                context: std::ptr::addr_of!(*context) as *mut _,
            };

            let mut error = CBLError::default();
            let replicator = CBLReplicator_Create(&cbl_config, std::ptr::addr_of_mut!(error));

            check_error(&error).map(move |_| Self {
                cbl_ref: replicator,
                config: Some(config),
                headers: Some(headers),
                context: Some(context),
                change_listeners: vec![],
                document_listeners: vec![],
            })
        }
    }

    /** Starts a replicator, asynchronously. Does nothing if it's already started. */
    pub fn start(&mut self, reset_checkpoint: bool) {
        unsafe {
            CBLReplicator_Start(self.get_ref(), reset_checkpoint);
        }
    }

    /** Stops a running replicator, asynchronously. Does nothing if it's not already started.
    The replicator will call your \ref CBLReplicatorChangeListener with an activity level of
    \ref kCBLReplicatorStopped after it stops. Until then, consider it still active.
    The parameter timout_seconds has a default value of 10. */
    pub fn stop(&mut self, timeout_seconds: Option<u64>) -> bool {
        unsafe {
            let timeout_seconds = timeout_seconds.unwrap_or(10);
            let (sender, receiver) = channel();
            let callback: ReplicatorChangeListener = Box::new(move |status| {
                if status.activity == ReplicatorActivityLevel::Stopped {
                    let _ = sender.send(true);
                }
            });

            let token = CBLReplicator_AddChangeListener(
                self.get_ref(),
                Some(c_replicator_change_listener),
                std::mem::transmute(&callback),
            );

            let mut success = true;
            if self.status().activity != ReplicatorActivityLevel::Stopped {
                CBLReplicator_Stop(self.get_ref());
                success = receiver
                    .recv_timeout(Duration::from_secs(timeout_seconds))
                    .is_ok();
            }
            CBLListener_Remove(token);
            success
        }
    }

    /** Informs the replicator whether it's considered possible to reach the remote host with
    the current network configuration. The default value is true. This only affects the
    replicator's behavior while it's in the Offline state:
    * Setting it to false will cancel any pending retry and prevent future automatic retries.
    * Setting it back to true will initiate an immediate retry.*/
    pub fn set_host_reachable(&mut self, reachable: bool) {
        unsafe {
            CBLReplicator_SetHostReachable(self.get_ref(), reachable);
        }
    }

    /** Puts the replicator in or out of "suspended" state. The default is false.
    * Setting suspended=true causes the replicator to disconnect and enter Offline state;
      it will not attempt to reconnect while it's suspended.
    * Setting suspended=false causes the replicator to attempt to reconnect, _if_ it was
      connected when suspended, and is still in Offline state. */
    pub fn set_suspended(&mut self, suspended: bool) {
        unsafe {
            CBLReplicator_SetSuspended(self.get_ref(), suspended);
        }
    }

    /** Returns the replicator's current status. */
    pub fn status(&self) -> ReplicatorStatus {
        unsafe { CBLReplicator_Status(self.get_ref()).into() }
    }

    /** Indicates which documents have local changes that have not yet been pushed to the server
    by this replicator. This is of course a snapshot, that will go out of date as the replicator
    makes progress and/or documents are saved locally. */
    pub fn pending_document_ids(&self) -> Result<HashSet<String>> {
        unsafe {
            let mut error = CBLError::default();
            let docs: FLDict =
                CBLReplicator_PendingDocumentIDs(self.get_ref(), std::ptr::addr_of_mut!(error));

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
            let result = CBLReplicator_IsDocumentPending(
                self.get_ref(),
                from_str(doc_id).get_ref(),
                std::ptr::addr_of_mut!(error),
            );
            check_error(&error).map(|_| result)
        }
    }

    /**
     Adds a listener that will be called when the replicator's status changes.
    */
    #[must_use]
    pub fn add_change_listener(mut self, listener: ReplicatorChangeListener) -> Self {
        let listener = unsafe {
            let listener = Box::new(listener);
            let ptr = Box::into_raw(listener);
            Listener::new(
                ListenerToken::new(CBLReplicator_AddChangeListener(
                    self.get_ref(),
                    Some(c_replicator_change_listener),
                    ptr.cast(),
                )),
                Box::from_raw(ptr),
            )
        };
        self.change_listeners.push(listener);
        self
    }

    /** Adds a listener that will be called when documents are replicated. */
    #[must_use]
    pub fn add_document_listener(mut self, listener: ReplicatedDocumentListener) -> Self {
        let listener = unsafe {
            let listener = Box::new(listener);
            let ptr = Box::into_raw(listener);

            Listener::new(
                ListenerToken::new(CBLReplicator_AddDocumentReplicationListener(
                    self.get_ref(),
                    Some(c_replicator_document_change_listener),
                    ptr.cast(),
                )),
                Box::from_raw(ptr),
            )
        };
        self.document_listeners.push(listener);
        self
    }
}

impl Drop for Replicator {
    fn drop(&mut self) {
        unsafe { release(self.get_ref()) }
    }
}

//======== STATUS AND PROGRESS

/** The possible states a replicator can be in during its lifecycle. */
#[derive(Debug, PartialEq, Eq)]
pub enum ReplicatorActivityLevel {
    Stopped,    // The replicator is unstarted, finished, or hit a fatal error.
    Offline,    // The replicator is offline, as the remote host is unreachable.
    Connecting, // The replicator is connecting to the remote host.
    Idle,       // The replicator is inactive, waiting for changes to sync.
    Busy,       // The replicator is actively transferring data.
}

impl From<u8> for ReplicatorActivityLevel {
    fn from(level: u8) -> Self {
        match u32::from(level) {
            kCBLReplicatorStopped => Self::Stopped,
            kCBLReplicatorOffline => Self::Offline,
            kCBLReplicatorConnecting => Self::Connecting,
            kCBLReplicatorIdle => Self::Idle,
            kCBLReplicatorBusy => Self::Busy,
            _ => unreachable!(),
        }
    }
}

/** The current progress status of a Replicator. The `fraction_complete` ranges from 0.0 to 1.0 as
replication progresses. The value is very approximate and may bounce around during replication;
making it more accurate would require slowing down the replicator and incurring more load on the
server. It's fine to use in a progress bar, though. */
#[derive(Debug)]
pub struct ReplicatorProgress {
    pub fraction_complete: f32, // Very-approximate completion, from 0.0 to 1.0
    pub document_count: u64,    // Number of documents transferred so far
}

/** A replicator's current status. */
#[derive(Debug)]
pub struct ReplicatorStatus {
    pub activity: ReplicatorActivityLevel, // Current state
    pub progress: ReplicatorProgress,      // Approximate fraction complete
    pub error: Result<()>,                 // Error, if any
}

impl From<CBLReplicatorStatus> for ReplicatorStatus {
    fn from(status: CBLReplicatorStatus) -> Self {
        Self {
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
pub type ReplicatorChangeListener = Box<dyn Fn(ReplicatorStatus)>;
#[no_mangle]
unsafe extern "C" fn c_replicator_change_listener(
    context: *mut ::std::os::raw::c_void,
    _replicator: *mut CBLReplicator,
    status: *const CBLReplicatorStatus,
) {
    let callback = context as *const ReplicatorChangeListener;
    let status: ReplicatorStatus = (*status).into();
    (*callback)(status);
}

/** A callback that notifies you when documents are replicated. */
pub type ReplicatedDocumentListener = Box<dyn Fn(Direction, Vec<ReplicatedDocument>)>;
unsafe extern "C" fn c_replicator_document_change_listener(
    context: *mut ::std::os::raw::c_void,
    _replicator: *mut CBLReplicator,
    is_push: bool,
    num_documents: u32,
    documents: *const CBLReplicatedDocument,
) {
    let callback = context as *const ReplicatedDocumentListener;

    let direction = if is_push {
        Direction::Pushed
    } else {
        Direction::Pulled
    };

    let repl_documents = std::slice::from_raw_parts(documents, num_documents as usize)
        .iter()
        .filter_map(|document| {
            document.ID.to_string().map(|doc_id| ReplicatedDocument {
                id: doc_id,
                flags: document.flags,
                error: check_error(&document.error),
            })
        })
        .collect();

    (*callback)(direction, repl_documents);
}

/** Flags describing a replicated document. */
pub static DELETED: u32 = kCBLDocumentFlagsDeleted;
pub static ACCESS_REMOVED: u32 = kCBLDocumentFlagsAccessRemoved;

/** Information about a document that's been pushed or pulled. */
pub struct ReplicatedDocument {
    pub id: String,        // The document ID
    pub flags: u32,        // Indicates whether the document was deleted or removed
    pub error: Result<()>, // Error, if document failed to replicate
}

/** Direction of document transfer. */
#[derive(Debug)]
pub enum Direction {
    Pulled,
    Pushed,
}
