#![allow(non_upper_case_globals)]

use std::collections::HashMap;
use crate::{
    CblRef, Database, MutableArray,
    c_api::{
        CBLReplicationCollection, CBLReplicatorType, kCBLReplicatorTypePull,
        kCBLReplicatorTypePush, kCBLReplicatorTypePushAndPull,
    },
    callbacks::{
        c_replication_conflict_resolver, c_replication_pull_filter, c_replication_push_filter,
    },
    collection::Collection,
    replication::{
        authenticator::Authenticator,
        callbacks::{ConflictResolver, ReplicationFilter},
        endpoint::Endpoint,
        proxy::ProxySettings,
    },
};

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

pub struct ReplicationCollection {
    pub collection: Collection,
    pub conflict_resolver: Option<ConflictResolver>, // Optional conflict-resolver callback.
    pub push_filter: Option<ReplicationFilter>, // Optional callback to filter which docs are pushed.
    pub pull_filter: Option<ReplicationFilter>, // Optional callback to validate incoming docs.
    pub channels: MutableArray,                 // Optional set of channels to pull from
    pub document_ids: MutableArray,             // Optional set of document IDs to replicate
}

impl ReplicationCollection {
    pub fn to_cbl_replication_collection(&self) -> CBLReplicationCollection {
        CBLReplicationCollection {
            collection: self.collection.get_ref(),
            conflictResolver: self
                .conflict_resolver
                .as_ref()
                .and(Some(c_replication_conflict_resolver)),
            pushFilter: self
                .push_filter
                .as_ref()
                .and(Some(c_replication_push_filter)),
            pullFilter: self
                .pull_filter
                .as_ref()
                .and(Some(c_replication_pull_filter)),
            channels: self.channels.get_ref(),
            documentIDs: self.document_ids.get_ref(),
        }
    }
}

/** The configuration of a replicator. */
pub struct ReplicatorConfiguration {
    pub database: Option<Database>, // The database to replicate. When setting the database, ONLY the default collection will be used for replication. (Required if collections is not set).
    pub endpoint: Endpoint,         // The address of the other database to replicate with
    pub replicator_type: ReplicatorType, // Push, pull or both
    pub continuous: bool,           // Continuous replication?
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

    pub collections: Option<Vec<ReplicationCollection>>, // The collections to replicate with the target's endpoint (Required if the database is not set).

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
