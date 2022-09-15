// A very simple program using Couchbase Lite
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
extern crate tempdir;

use self::couchbase_lite::*;
use std::thread;
use std::time::Duration;

use tempdir::TempDir;

pub const DB_NAME: &str = "test_db";

const LEVEL_PREFIX: [&str; 5] = ["((", "_", "", "WARNING: ", "***ERROR: "];
const LEVEL_SUFFIX: [&str; 5] = ["))", "_", "", "", " ***"];

fn logger(domain: logging::Domain, level: logging::Level, message: &str) {
    // Log to stdout, not stderr, so that `cargo test` will buffer the output.
    let i = level as usize;
    println!(
        "CBL {:?}: {}{}{}",
        domain, LEVEL_PREFIX[i], message, LEVEL_SUFFIX[i]
    )
}

pub fn with_db<F>(f: F)
where
    F: Fn(&mut Database),
{
    logging::set_callback(Some(logger));
    logging::set_callback_level(logging::Level::Verbose);
    logging::set_console_level(logging::Level::None);

    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: None,
    };
    let mut db = Database::open(DB_NAME, Some(cfg)).expect("open db");
    assert!(Database::exists(DB_NAME, tmp_dir.path()));

    f(&mut db);

    db.delete().unwrap();
}

fn main() {
    with_db(|db| {
        // Start replication
        let token = "test_token";
        let endpoint1 = Endpoint::new_with_url("ws://localhost:4984/billeo-db/").unwrap();
        let endpoint2 = Endpoint::new_with_url("ws://localhost:4984/billeo-db/").unwrap();

        let config1 = ReplicatorConfiguration {
            database: db.clone(),
            endpoint: endpoint1,
            replicator_type: ReplicatorType::PushAndPull,
            continuous: true,
            disable_auto_purge: true,
            max_attempts: 4,
            max_attempt_wait_time: 100,
            heartbeat: 120,
            authenticator: None,
            proxy: None,
            headers: vec![(
                "Cookie".to_string(),
                format!("SyncGatewaySession={}", token),
            )]
            .into_iter()
            .collect(),
            pinned_server_certificate: None,
            trusted_root_certificates: None,
            channels: MutableArray::default(),
            document_ids: MutableArray::default(),
        };
        let config2 = ReplicatorConfiguration {
            database: db.clone(),
            endpoint: endpoint2,
            replicator_type: ReplicatorType::PushAndPull,
            continuous: true,
            disable_auto_purge: true,
            max_attempts: 4,
            max_attempt_wait_time: 100,
            heartbeat: 120,
            authenticator: None,
            proxy: None,
            headers: vec![(
                "Cookie".to_string(),
                format!("SyncGatewaySession={}", token),
            )]
            .into_iter()
            .collect(),
            pinned_server_certificate: None,
            trusted_root_certificates: None,
            channels: MutableArray::default(),
            document_ids: MutableArray::default(),
        };
        let context1 = ReplicationConfigurationContext {
            push_filter: None,
            pull_filter: None,
            conflict_resolver: None,
            property_encryptor: None,
            property_decryptor: None,
        };
        let context2 = ReplicationConfigurationContext {
            push_filter: None,
            pull_filter: None,
            conflict_resolver: None,
            property_encryptor: None,
            property_decryptor: None,
        };

        let mut repl1 = Replicator::new(config1, Box::new(context1)).unwrap();
        let mut repl2 = Replicator::new(config2, Box::new(context2)).unwrap();

        thread::spawn(move || loop {
            repl1.start(false);
            thread::sleep(Duration::from_millis(100));
            repl1.stop();
        });
        thread::spawn(move || loop {
            repl2.start(false);
            thread::sleep(Duration::from_millis(150));
            repl2.stop();
        });
        thread::sleep(Duration::from_millis(100000));
    });
}
