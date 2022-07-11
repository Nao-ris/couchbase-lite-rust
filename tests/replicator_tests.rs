// Couchbase Lite unit tests
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

extern crate couchbase_lite;
extern crate lazy_static;

use self::couchbase_lite::*;
use lazy_static::lazy_static;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub mod utils;

//////// TESTS:

#[test]
fn config() {
    utils::with_db(|db| {
        let repl_config_in = ReplicatorConfiguration {
            database: db.clone(),
            endpoint: Endpoint::new_with_url("ws://localhost:4984/billeo-db".to_string()).unwrap(),
            replicator_type: ReplicatorType::PushAndPull,
            continuous: true,
            disable_auto_purge: true,
            max_attempts: 4,
            max_attempt_wait_time: 100,
            heartbeat: 120,
            authenticator: Some(Authenticator::create_session("session_id".to_string(), "cookie_name".to_string())),
            proxy: Some(ProxySettings {
                proxy_type: ProxyType::HTTP,
                hostname: Some("hostname".to_string()),
                port: 3000,
                username: Some("username".to_string()),
                password: Some("password".to_string()),
            }),
            headers: HashMap::new(),
            pinned_server_certificate: None,
            trusted_root_certificates: None,
            channels: Array::default(),
            document_ids: Array::default(),
            push_filter: None,
            pull_filter: None,
            conflict_resolver: None,
            property_encryptor: None,
            property_decryptor: None,
        };

        let repl = Replicator::new(repl_config_in).unwrap();

        let repl_config_out = repl.config().unwrap();

        assert_eq!(repl_config_out.database, db.clone());
        assert_eq!(repl_config_out.replicator_type, ReplicatorType::PushAndPull);
        assert_eq!(repl_config_out.continuous, true);
        assert_eq!(repl_config_out.disable_auto_purge, true);
        assert_eq!(repl_config_out.max_attempts, 4);
        assert_eq!(repl_config_out.max_attempt_wait_time, 100);
        assert_eq!(repl_config_out.heartbeat, 120);
        let proxy = repl_config_out.proxy.unwrap();
        assert_eq!(proxy.proxy_type, ProxyType::HTTP);
        assert_eq!(proxy.hostname, Some("hostname".to_string()));
        assert_eq!(proxy.port, 3000);
        assert_eq!(proxy.username, Some("username".to_string()));
        assert_eq!(proxy.password, Some("password".to_string()));
        assert_eq!(repl_config_out.headers, HashMap::new());
    });
}

#[test]
fn basic_local_replication() {
    let config1: utils::ReplicationTestConfiguration = Default::default();
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(config1, config2, |local_db1, local_db2, central_db, _repl1, _repl2| {
        // Save doc
        utils::add_doc(local_db1, "foo", 1234, "Hello World!");

        // Check if replication to central
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));

        // Check if replication to DB 2
        assert!(utils::check_callback_with_wait(|| local_db2.get_document("foo").is_ok(), None));
    });
}

#[test]
fn pull_type_not_pushing() {
    let config1 = utils::ReplicationTestConfiguration {
        replicator_type: ReplicatorType::Pull,
        ..Default::default()
    };
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(config1, config2, |local_db1, _local_db2, central_db, _repl1, _repl2| {
        // Save doc
        utils::add_doc(local_db1, "foo", 1234, "Hello World!");

        // Check the replication process is not pushing to central
        assert!(!utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
    });
}

#[test]
fn push_type_not_pulling() {
    let config1 = Default::default();
    let config2 = utils::ReplicationTestConfiguration {
        replicator_type: ReplicatorType::Push,
        ..Default::default()
    };

    utils::with_three_dbs(config1, config2, |local_db1, local_db2, central_db, _repl1, _repl2| {
        // Save doc
        utils::add_doc(local_db1, "foo", 1234, "Hello World!");

        // Check if replication to central
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));

        // Check the replication process is not pulling to DB 2
        assert!(!utils::check_callback_with_wait(|| local_db2.get_document("foo").is_ok(), None));
    });
}

#[test]
fn continuous() {
    let config1 = utils::ReplicationTestConfiguration {
        continuous: false,
        ..Default::default()
    };
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(config1, config2, |local_db1, _local_db2, central_db, repl1, _repl2| {
        // Save doc
        utils::add_doc(local_db1, "foo", 1234, "Hello World!");

        // Check the replication process is not running automatically
        assert!(!utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));

        // Manually trigger the replication
        repl1.start(false);

        // Check the replication was successful
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
    });
}

#[test]
fn document_ids() {
    let mut array = MutableArray::new();
    array.append().put_string("foo");
    array.append().put_string("foo3");
    let config1 = utils::ReplicationTestConfiguration {
        document_ids: array.as_array(),
        ..Default::default()
    };
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(config1, config2, |local_db1, _local_db2, central_db, _repl1, _repl2| {
        // Save doc 'foo' and 'foo2'
        utils::add_doc(local_db1, "foo", 1234, "Hello World!");
        utils::add_doc(local_db1, "foo2", 1234, "Hello World!");

        // Check only foo is replicated
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
        assert!(!utils::check_callback_with_wait(|| central_db.get_document("foo2").is_ok(), None));
    });
}

#[test]
fn push_and_pull_filter() {
    let config1 = utils::ReplicationTestConfiguration {
        push_filter: Some(|document, _is_deleted, _is_access_removed| document.id() == "foo" || document.id() == "foo2"),
        ..Default::default()
    };
    let config2 = utils::ReplicationTestConfiguration {
        pull_filter: Some(|document, _is_deleted, _is_access_removed| document.id() == "foo2" || document.id() == "foo3"),
        ..Default::default()
    };

    utils::with_three_dbs(config1, config2, |local_db1, local_db2, central_db, _repl1, _repl2| {
        // Save doc 'foo', 'foo2' & 'foo3'
        utils::add_doc(local_db1, "foo", 1234, "Hello World!");
        utils::add_doc(local_db1, "foo2", 1234, "Hello World!");
        utils::add_doc(local_db1, "foo3", 1234, "Hello World!");

        // Check only 'foo' and 'foo2' were replicated to central
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo2").is_ok(), None));
        assert!(!utils::check_callback_with_wait(|| central_db.get_document("foo3").is_ok(), None));

        // Check only foo2' were replicated to DB 2
        assert!(!utils::check_callback_with_wait(|| local_db2.get_document("foo").is_ok(), None));
        assert!(utils::check_callback_with_wait(|| local_db2.get_document("foo2").is_ok(), None));
    });
}

lazy_static! {
    static ref CONFLICT_DETECTED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[test]
fn conflict_resolver() {
    utils::set_static(&CONFLICT_DETECTED, false);

    let config1 = utils::ReplicationTestConfiguration {
        conflict_resolver: Some(|_document_id, _local_document, remote_document| {
            utils::set_static(&CONFLICT_DETECTED, true);
            remote_document
        }),
        ..Default::default()
    };
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(config1, config2, |local_db1, local_db2, central_db, repl1, _repl2| {
        let i = 1234;
        let i1 = 1;
        let i2 = 2;

        // Save doc 'foo'
        utils::add_doc(local_db1, "foo", i, "Hello World!");

        // Check 'foo' is replicated to central and DB 2
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
        assert!(utils::check_callback_with_wait(|| local_db2.get_document("foo").is_ok(), None));

        // Stop replication on DB 1
        repl1.stop();

        // Modify 'foo' in DB 1
        let mut foo = local_db1.get_document("foo").unwrap();
        foo.mutable_properties().at("i").put_i64(i1);
        local_db1.save_document(&mut foo, ConcurrencyControl::FailOnConflict).expect("save");

        // Modify 'foo' in DB 2
        let mut foo = local_db2.get_document("foo").unwrap();
        foo.mutable_properties().at("i").put_i64(i2);
        local_db2.save_document(&mut foo, ConcurrencyControl::FailOnConflict).expect("save");

        // Check DB 2 version is in central
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").unwrap().properties().get("i").as_i64_or_0() == i2, None));

        // Restart DB 1 replication
        repl1.start(false);

        // Check conflict was detected
        assert!(utils::check_static_with_wait(&CONFLICT_DETECTED, true, None));

        // Check DB 2 version is in DB 1
        assert!(utils::check_callback_with_wait(|| local_db1.get_document("foo").unwrap().properties().get("i").as_i64_or_0() == i2, None));
    });
}
