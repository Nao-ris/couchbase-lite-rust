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

pub mod utils;

use std::{ thread, time };

//////// TESTS:

#[test]
fn config() {
    utils::with_db(|db| {
        let headers_mut = MutableDict::new();

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
            headers: headers_mut.as_dict(),
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
        assert_eq!(repl_config_out.headers, headers_mut.as_dict());
    });
}

#[test]
fn basic_local_replication() {
    utils::with_three_dbs(|local_db1, _local_db2, central_db| {
        // Create replicator
        let headers_mut = MutableDict::new();

        let repl_config = ReplicatorConfiguration {
            database: local_db1.clone(),
            endpoint: Endpoint::new_with_local_db(central_db),
            replicator_type: ReplicatorType::PushAndPull,
            continuous: true,
            disable_auto_purge: true,
            max_attempts: 4,
            max_attempt_wait_time: 100,
            heartbeat: 120,
            authenticator: None,
            proxy: None,
            headers: headers_mut.as_dict(),
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

        let mut repl = Replicator::new(repl_config).unwrap();

        // Start replication
        repl.start(false);

        // Save doc
        let mut doc = Document::new_with_id("foo");
        let mut props = doc.mutable_properties();
        props.at("i").put_i64(1234);
        props.at("s").put_string("Hello World!");

        local_db1.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");

        // Check if replication
        let ten_seconds = time::Duration::from_secs(10);
        let now = time::Instant::now();
        let wait_fetch_document = time::Duration::from_millis(1000);

        let mut doc_found = false;
        while !doc_found && now.elapsed() < ten_seconds {
            let doc = central_db.get_document("foo");
            doc_found = doc.is_ok();
            thread::sleep(wait_fetch_document);
        }

        assert_eq!(doc_found, true);

        // Stop replication
        repl.stop();
    });
}
