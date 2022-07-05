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

use std::collections::HashMap;
use std::collections::HashSet;

use super::*;
use super::c_api::*;

// WARNING: THIS API IS UNIMPLEMENTED SO FAR


//======== CONFIGURATION


/** Represents the location of a database to replicate with. */
pub enum Endpoint<'e> {
    WithURL     (String),
    WithLocalDB (&'e Database)
}


pub enum Authenticator<'a> {
    None,
    Basic   {username: &'a str, password: &'a str},
    Session {session_id: &'a str},
    Cookie  {name: &'a str, value: &'a str}
}


/** Direction of replication: push, pull, or both. */
#[derive(Debug)]
pub enum ReplicatorType { PushAndPull, Push, Pull }


/** Types of proxy servers, for CBLProxySettings. */
#[derive(Debug)]
pub enum ProxyType { HTTP, HTTPS }

/** Proxy settings for the replicator. */
pub struct ProxySettings<'p> {
    pub proxy_type: ProxyType,          // Type of proxy
    pub hostname:   &'p str,            // Proxy server hostname or IP address
    pub port:       u16,                // Proxy server port
    pub username:   Option<&'p str>,    // Username for proxy auth
    pub password:   Option<&'p str>     // Password for proxy auth
}


/** A callback that can decide whether a particular document should be pushed or pulled. */
pub type ReplicationFilter =  fn(document: &Document,
                                 is_deleted: bool) -> bool;

/** Conflict-resolution callback for use in replications. This callback will be invoked
    when the replicator finds a newer server-side revision of a document that also has local
    changes. The local and remote changes must be resolved before the document can be pushed
    to the server. */
pub type ConflictResolver = fn(document_id: &str,
                               local_document: Option<Document>,
                               remote_document: Option<Document>) -> Document;


/** The configuration of a replicator. */
pub struct ReplicatorConfiguration<'c> {
    pub database:                  &'c Database,            // The database to replicate
    pub endpoint:                  Endpoint<'c>,    // The address of the other database to replicate with
    pub replicator_type:           ReplicatorType,          // Push, pull or both
    pub continuous:                bool,                    // Continuous replication?
    pub authenticator:             Authenticator<'c>,   // Authentication credentials, if needed
    pub proxy:                     Option<ProxySettings<'c>>,   // HTTP client proxy settings
    pub headers:                   Option<HashMap<&'c str,&'c str>>, // Extra HTTP headers to add to the WebSocket request
    pub pinned_server_certificate: Option<Vec<u8>>,         // An X.509 cert to "pin" TLS connections to (PEM or DER)
    pub trusted_root_certificates: Option<Vec<u8>>,         // Set of anchor certs (PEM format)
    pub channels:                  Option<Vec<&'c str>>,    // Optional set of channels to pull from
    pub document_ids:              Option<Vec<&'c str>>,    // Optional set of document IDs to replicate
    pub push_filter:               ReplicationFilter,       // Optional callback to filter which docs are pushed
    pub pull_filter:               ReplicationFilter,       // Optional callback to validate incoming docs
    pub conflict_resolver:         ConflictResolver,        // Optional conflict-resolver callback
}


//======== LIFECYCLE

/** A background task that syncs a \ref Database with a remote server or peer. */
pub struct Replicator {
    _ref: *mut CBLReplicator,
    has_ownership: bool,
}

impl Replicator {
    /** Creates a replicator with the given configuration. */
    pub fn new(_config: ReplicatorConfiguration) -> Result<Replicator> {
        todo!()
    }

    /** Returns the configuration of an existing replicator. */
    pub fn config(&self) -> ReplicatorConfiguration {
        todo!()
    }

    /** Instructs the replicator to ignore existing checkpoints the next time it runs.
        This will cause it to scan through all the documents on the remote database, which takes
        a lot longer, but it can resolve problems with missing documents if the client and
        server have gotten out of sync somehow. */
    pub fn reset_checkpoint(&mut self) {
        todo!()
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
            if self.has_ownership {
                CBL_Release(self._ref as *mut CBLRefCounted)
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

    let replicator = Replicator {
        _ref: replicator,
        has_ownership: false,
    };
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

    let replicator = Replicator {
        _ref: replicator,
        has_ownership: false,
    };
    let direction = if is_push { Direction::Pushed } else { Direction::Pulled};

    let mut vec_repl_docs = Vec::new();
    for i in 0..num_documents {
        if let Some(document) = documents.offset(i as isize).as_ref() {
            if let Some(doc_id) = document.ID.to_string() {
                vec_repl_docs.push(ReplicatedDocument {
                    id: doc_id,
                    flags: document.flags,
                    error: check_error(&document.error),
                })
            }
        }
    }

    callback(&replicator, direction, vec_repl_docs);
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
