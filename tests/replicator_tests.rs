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

use self::couchbase_lite::*;

use std::collections::HashMap;

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
        let mut doc = Document::new_with_id("foo");
        let mut props = doc.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");

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
        let mut doc = Document::new_with_id("foo");
        let mut props = doc.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");

        // Check the replication process is not pushing to central
        assert!(!utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
    });
}

#[test]
fn push_type_not_pulling() {
    let config1 = Default::default();
    let config2: utils::ReplicationTestConfiguration = utils::ReplicationTestConfiguration {
        replicator_type: ReplicatorType::Push,
        ..Default::default()
    };

    utils::with_three_dbs(config1, config2, |local_db1, local_db2, central_db, _repl1, _repl2| {
        // Save doc
        let mut doc = Document::new_with_id("foo");
        let mut props = doc.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");

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
        let mut doc = Document::new_with_id("foo");
        let mut props = doc.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");

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
        // Save doc 'foo'
        let mut doc = Document::new_with_id("foo");
        let mut props = doc.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");

        // Save doc 'foo2'
        let mut doc2 = Document::new_with_id("foo2");
        let mut props = doc2.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc2, ConcurrencyControl::FailOnConflict).expect("save");

        // Check the replication process is not running automatically
        assert!(utils::check_callback_with_wait(|| central_db.get_document("foo").is_ok(), None));
        assert!(!utils::check_callback_with_wait(|| central_db.get_document("foo2").is_ok(), None));
    });
}
