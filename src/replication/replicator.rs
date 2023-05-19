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

use std::{ptr, collections::HashSet, sync::mpsc::channel, time::Duration};
use crate::{
    CblRef, Dict, Error, Listener, ListenerToken, MutableDict, Result, check_error, release,
    retain,
    c_api::{
        CBLListener_Remove, CBLError, CBLReplicatedDocument, CBLReplicator,
        CBLReplicatorConfiguration, CBLReplicatorStatus, CBLReplicator_AddChangeListener,
        CBLReplicator_AddDocumentReplicationListener, CBLReplicator_Create,
        CBLReplicator_IsDocumentPending, CBLReplicator_PendingDocumentIDs,
        CBLReplicator_SetHostReachable, CBLReplicator_SetSuspended, CBLReplicator_Start,
        CBLReplicator_Status, CBLReplicator_Stop, FLDict, kCBLReplicatorBusy,
        kCBLReplicatorConnecting, kCBLReplicatorIdle, kCBLReplicatorOffline, kCBLReplicatorStopped,
        CBLReplicationCollection,
    },
    replication::{
        callbacks::{
            ReplicationConfigurationContext, c_property_decryptor, c_property_encryptor,
            c_replication_conflict_resolver, c_replication_pull_filter, c_replication_push_filter,
        },
        configuration::ReplicatorConfiguration,
    },
    slice::{from_str, self},
};

type ReplicatorsListeners<T> = Vec<Listener<Box<T>>>;

/** A background task that syncs a \ref Database with a remote server or peer. */
pub struct Replicator {
    cbl_ref: *mut CBLReplicator,
    pub config: Option<ReplicatorConfiguration>,
    pub headers: Option<MutableDict>,
    pub context: Option<Box<ReplicationConfigurationContext>>,
    change_listeners: ReplicatorsListeners<ReplicatorChangeListener>,
    _collections: Option<Vec<CBLReplicationCollection>>,
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
            let mut collections: Option<Vec<CBLReplicationCollection>> =
                config.collections.as_ref().map(|collections| {
                    collections
                        .iter()
                        .map(|c| c.to_cbl_replication_collection())
                        .collect()
                });

            let cbl_config = CBLReplicatorConfiguration {
                database: config
                    .database
                    .as_ref()
                    .map(|d| retain(d.get_ref()))
                    .unwrap_or(ptr::null_mut()),
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
                documentPropertyEncryptor: None,
                documentPropertyDecryptor: None,
                collections: if let Some(collections) = collections.as_mut() {
                    collections.as_mut_ptr()
                } else {
                    ptr::null_mut()
                },
                collectionCount: collections.as_ref().map(|c| c.len()).unwrap_or_default(),
                acceptParentDomainCookies: config.accept_parent_domain_cookies,
                context: std::ptr::addr_of!(*context) as *mut _,
            };

            let mut error = CBLError::default();
            let replicator = CBLReplicator_Create(&cbl_config, std::ptr::addr_of_mut!(error));

            check_error(&error).map(move |_| Self {
                cbl_ref: replicator,
                _collections: collections,
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
