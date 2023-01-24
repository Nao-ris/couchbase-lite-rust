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
use encryptable::Encryptable;
use std::{time::Duration, thread};

pub mod utils;

//////// TESTS:

#[test]
fn basic_local_replication() {
    let config1: utils::ReplicationTestConfiguration = Default::default();
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(ReplicationConfigurationContext::default()),
        Box::new(ReplicationConfigurationContext::default()),
        |local_db1, local_db2, central_db, _repl1, _repl2| {
            // Save doc
            utils::add_doc(local_db1, "foo", 1234, "Hello World!");

            // Check if replication to central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));

            // Check if replication to DB 2
            assert!(utils::check_callback_with_wait(
                || local_db2.get_document("foo").is_ok(),
                None
            ));
        },
    );
}

#[test]
fn pull_type_not_pushing() {
    let config1 = utils::ReplicationTestConfiguration {
        replicator_type: ReplicatorType::Pull,
        ..Default::default()
    };
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(ReplicationConfigurationContext::default()),
        Box::new(ReplicationConfigurationContext::default()),
        |local_db1, local_db2, central_db, _repl1, _repl2| {
            // Save doc in DB 1
            utils::add_doc(local_db1, "foo", 1234, "Hello World!");

            // Check the replication process is not pushing to central
            assert!(!utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));

            // Save doc in DB 2
            utils::add_doc(local_db2, "foo2", 1234, "Hello World!");

            // Check 'foo2' is pulled in DB 1
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo2").is_ok(),
                None
            ));
            assert!(utils::check_callback_with_wait(
                || local_db1.get_document("foo2").is_ok(),
                None
            ));
        },
    );
}

#[test]
fn push_type_not_pulling() {
    let config1 = Default::default();
    let config2 = utils::ReplicationTestConfiguration {
        replicator_type: ReplicatorType::Push,
        ..Default::default()
    };

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(ReplicationConfigurationContext::default()),
        Box::new(ReplicationConfigurationContext::default()),
        |local_db1, local_db2, central_db, _repl1, _repl2| {
            // Save doc in DB 1
            utils::add_doc(local_db1, "foo", 1234, "Hello World!");

            // Check if replication to central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));

            // Check the replication process is not pulling to DB 2
            assert!(!utils::check_callback_with_wait(
                || local_db2.get_document("foo").is_ok(),
                None
            ));

            // Save doc in DB 2
            utils::add_doc(local_db2, "foo2", 1234, "Hello World!");

            // Check 'foo2' is pushed in central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo2").is_ok(),
                None
            ));
        },
    );
}

#[test]
fn document_ids() {
    let mut document_ids = MutableArray::new();
    document_ids.append().put_string("foo");
    document_ids.append().put_string("foo3");
    let config1 = utils::ReplicationTestConfiguration {
        document_ids,
        ..Default::default()
    };
    let config2: utils::ReplicationTestConfiguration = Default::default();

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(ReplicationConfigurationContext::default()),
        Box::new(ReplicationConfigurationContext::default()),
        |local_db1, _local_db2, central_db, _repl1, _repl2| {
            // Save doc 'foo' and 'foo2'
            utils::add_doc(local_db1, "foo", 1234, "Hello World!");
            utils::add_doc(local_db1, "foo2", 1234, "Hello World!");

            // Check only foo is replicated
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));
            assert!(!utils::check_callback_with_wait(
                || central_db.get_document("foo2").is_ok(),
                None
            ));
        },
    );
}

#[test]
fn push_and_pull_filter() {
    let config1 = utils::ReplicationTestConfiguration::default();
    let config2 = utils::ReplicationTestConfiguration::default();

    let context1 = ReplicationConfigurationContext {
        push_filter: Some(Box::new(|document, _is_deleted, _is_access_removed| {
            document.id() == "foo" || document.id() == "foo2"
        })),
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: None,
        property_decryptor: None,
    };

    let context2 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: Some(Box::new(|document, _is_deleted, _is_access_removed| {
            document.id() == "foo2" || document.id() == "foo3"
        })),
        conflict_resolver: None,
        property_encryptor: None,
        property_decryptor: None,
    };

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, local_db2, central_db, _repl1, _repl2| {
            // Save doc 'foo', 'foo2' & 'foo3'
            utils::add_doc(local_db1, "foo", 1234, "Hello World!");
            utils::add_doc(local_db1, "foo2", 1234, "Hello World!");
            utils::add_doc(local_db1, "foo3", 1234, "Hello World!");

            // Check only 'foo' and 'foo2' were replicated to central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo2").is_ok(),
                None
            ));
            assert!(!utils::check_callback_with_wait(
                || central_db.get_document("foo3").is_ok(),
                None
            ));

            // Check only foo2' were replicated to DB 2
            assert!(!utils::check_callback_with_wait(
                || local_db2.get_document("foo").is_ok(),
                None
            ));
            assert!(utils::check_callback_with_wait(
                || local_db2.get_document("foo2").is_ok(),
                None
            ));
        },
    );
}

#[test]
fn conflict_resolver() {
    let (sender, receiver) = std::sync::mpsc::channel();

    let config1 = utils::ReplicationTestConfiguration::default();
    let config2 = utils::ReplicationTestConfiguration::default();

    let context1 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: Some(Box::new(
            move |_document_id, _local_document, remote_document| {
                sender.send(true).unwrap();
                remote_document
            },
        )),
        property_encryptor: None,
        property_decryptor: None,
    };

    let context2 = ReplicationConfigurationContext::default();

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, local_db2, central_db, repl1, _repl2| {
            let i = 1234;
            let i1 = 1;
            let i2 = 2;

            // Save doc 'foo'
            utils::add_doc(local_db1, "foo", i, "Hello World!");

            // Check 'foo' is replicated to central and DB 2
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));
            assert!(utils::check_callback_with_wait(
                || local_db2.get_document("foo").is_ok(),
                None
            ));

            // Stop replication on DB 1
            repl1.stop();

            // Modify 'foo' in DB 1
            let mut foo = local_db1.get_document("foo").unwrap();
            foo.mutable_properties().at("i").put_i64(i1);
            local_db1
                .save_document_with_concurency_control(&mut foo, ConcurrencyControl::FailOnConflict)
                .expect("save");

            // Modify 'foo' in DB 2
            let mut foo = local_db2.get_document("foo").unwrap();
            foo.mutable_properties().at("i").put_i64(i2);
            local_db2
                .save_document_with_concurency_control(&mut foo, ConcurrencyControl::FailOnConflict)
                .expect("save");

            // Check DB 2 version is in central
            assert!(utils::check_callback_with_wait(
                || central_db
                    .get_document("foo")
                    .unwrap()
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i2,
                None
            ));

            // Restart DB 1 replication
            repl1.start(false);

            // Check conflict was detected
            receiver.recv_timeout(Duration::from_secs(1)).unwrap();

            // Check DB 2 version is in DB 1
            assert!(utils::check_callback_with_wait(
                || local_db1
                    .get_document("foo")
                    .unwrap()
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i2,
                None
            ));
        },
    );
}

#[test]
fn conflict_resolver_save_keep_local() {
    let config1 = utils::ReplicationTestConfiguration::default();
    let config2 = utils::ReplicationTestConfiguration::default();

    let context1 = ReplicationConfigurationContext::default();
    let context2 = ReplicationConfigurationContext::default();

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, _local_db2, central_db, repl1, _repl2| {
            let i = 1234;
            let i1 = 1;
            let i2 = 2;

            // Save doc 'foo'
            utils::add_doc(local_db1, "foo", i, "Hello World!");

            // Check 'foo' is replicated to central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));

            // Stop replication on DB 1
            repl1.stop();

            // Modify 'foo' in central
            let mut foo = central_db.get_document("foo").unwrap();
            foo.mutable_properties().at("i").put_i64(i2);
            central_db
                .save_document_with_concurency_control(&mut foo, ConcurrencyControl::FailOnConflict)
                .expect("save");

            // Fetch 'foo' in DB 1
            let mut foo = local_db1.get_document("foo").unwrap();

            // Restart replication
            repl1.start(false);

            // Check central version of 'foo' is replicated to DB 1
            assert!(utils::check_callback_with_wait(
                || local_db1
                    .get_document("foo")
                    .expect("foo exists")
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i2,
                None
            ));

            // Modify 'foo' in DB1 from outdated document
            foo.mutable_properties().at("i").put_i64(i1);
            assert!(local_db1
                .save_document_resolving(&mut foo, move |_, _| true)
                .is_ok());

            // Assert conflict was resolved by keeping latest version
            assert!(utils::check_callback_with_wait(
                || local_db1
                    .get_document("foo")
                    .expect("foo exists")
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i1,
                None
            ));

            // Check 'foo' new version replicated to central
            assert!(utils::check_callback_with_wait(
                || central_db
                    .get_document("foo")
                    .expect("foo exists")
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i1,
                None
            ));
        },
    );
}

#[test]
fn conflict_resolver_save_keep_remote() {
    let config1 = utils::ReplicationTestConfiguration::default();
    let config2 = utils::ReplicationTestConfiguration::default();

    let context1 = ReplicationConfigurationContext::default();
    let context2 = ReplicationConfigurationContext::default();

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, _local_db2, central_db, repl1, _repl2| {
            let i = 1234;
            let i1 = 1;
            let i2 = 2;

            // Save doc 'foo'
            utils::add_doc(local_db1, "foo", i, "Hello World!");

            // Check 'foo' is replicated to central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));

            // Stop replication on DB 1
            repl1.stop();

            // Modify 'foo' in central
            let mut foo = central_db.get_document("foo").unwrap();
            foo.mutable_properties().at("i").put_i64(i2);
            central_db
                .save_document_with_concurency_control(&mut foo, ConcurrencyControl::FailOnConflict)
                .expect("save");

            // Fetch 'foo' in DB 1
            let mut foo = local_db1.get_document("foo").unwrap();

            // Restart replication
            repl1.start(false);

            // Check central version of 'foo' is replicated to DB 1
            assert!(utils::check_callback_with_wait(
                || local_db1
                    .get_document("foo")
                    .expect("foo exists")
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i2,
                None
            ));

            // Modify 'foo' in DB1 from outdated document
            foo.mutable_properties().at("i").put_i64(i1);
            assert!(local_db1
                .save_document_resolving(&mut foo, move |_, _| false)
                .is_err());

            // Assert conflict was resolved by keeping central's version
            assert!(utils::check_callback_with_wait(
                || local_db1
                    .get_document("foo")
                    .expect("foo exists")
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i2,
                None
            ));

            // Check 'foo' was unchanged in central
            assert!(utils::check_callback_with_wait(
                || central_db
                    .get_document("foo")
                    .expect("foo exists")
                    .properties()
                    .get("i")
                    .as_i64_or_0()
                    == i2,
                None
            ));
        },
    );
}

fn encryptor(
    _document_id: Option<String>,
    _properties: Dict,
    _key_path: Option<String>,
    input: Vec<u8>,
    _algorithm: Option<String>,
    _kid: Option<String>,
    _error: &Error,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(input.iter().map(|u| u ^ 48).collect())
}
fn decryptor(
    _document_id: Option<String>,
    _properties: Dict,
    _key_path: Option<String>,
    input: Vec<u8>,
    _algorithm: Option<String>,
    _kid: Option<String>,
    _error: &Error,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(input.iter().map(|u| u ^ 48).collect())
}
fn encryptor_err(
    _document_id: Option<String>,
    _properties: Dict,
    _key_path: Option<String>,
    _: Vec<u8>,
    _algorithm: Option<String>,
    _kid: Option<String>,
    _error: &Error,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    Err("".into())
}
fn decryptor_err(
    _document_id: Option<String>,
    _properties: Dict,
    _key_path: Option<String>,
    _: Vec<u8>,
    _algorithm: Option<String>,
    _kid: Option<String>,
    _error: &Error,
) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    Err("".into())
}

#[test]
fn encryption_ok_decryption_ok() {
    let config1 = utils::ReplicationTestConfiguration::default();
    let config2 = utils::ReplicationTestConfiguration::default();

    let context1 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: Some(encryptor),
        property_decryptor: Some(decryptor),
    };

    let context2 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: Some(encryptor),
        property_decryptor: Some(decryptor),
    };

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, local_db2, central_db, _repl1, _repl2| {
            // Save doc 'foo' with an encryptable property
            {
                let mut doc_db1 = Document::new_with_id("foo");
                let mut props = doc_db1.mutable_properties();
                props.at("i").put_i64(1234);
                props
                    .at("s")
                    .put_encrypt(&Encryptable::create_with_string("test_encryption"));
                local_db1
                    .save_document_with_concurency_control(
                        &mut doc_db1,
                        ConcurrencyControl::FailOnConflict,
                    )
                    .expect("save");
            }

            // Check document is replicated with data encrypted in central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));
            {
                let doc_central = central_db.get_document("foo").unwrap();
                let dict = doc_central.properties();
                assert!(dict.to_keys_hash_set().get("encrypted$s").is_some());
            }

            // Check document is replicated with data decrypted in DB 2
            assert!(utils::check_callback_with_wait(
                || local_db2.get_document("foo").is_ok(),
                None
            ));
            {
                let doc_db2 = local_db2.get_document("foo").unwrap();
                let dict = doc_db2.properties();
                let value = dict.get("s");
                assert!(value.is_encryptable());
                let encryptable = value.get_encryptable_value();
                assert!(encryptable.get_value().as_string() == Some("test_encryption"));
                drop(encryptable);
            }
        },
    );
}

#[test]
fn encryption_error() {
    let config1 = utils::ReplicationTestConfiguration {
        continuous: false,
        ..Default::default()
    };
    let config2 = utils::ReplicationTestConfiguration::default();

    let context1 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: Some(encryptor_err),
        property_decryptor: Some(decryptor),
    };

    let context2 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: Some(encryptor),
        property_decryptor: Some(decryptor),
    };

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, _local_db2, central_db, repl1, _repl2| {
            // Save doc 'foo' with an encryptable property
            {
                let mut doc_db1 = Document::new_with_id("foo");
                let mut props = doc_db1.mutable_properties();
                props.at("i").put_i64(1234);
                props
                    .at("s")
                    .put_encrypt(&Encryptable::create_with_string("test_encryption"));
                local_db1
                    .save_document_with_concurency_control(
                        &mut doc_db1,
                        ConcurrencyControl::FailOnConflict,
                    )
                    .expect("save");
            }

            // Manually trigger the replication
            repl1.start(false);

            // Check document is not replicated in central because of the encryption error
            thread::sleep(Duration::from_secs(5));
            assert!(central_db.get_document("foo").is_err());

            // Manually trigger the replication
            repl1.start(false); // the push will be retried even if 'reset_checkpoint = false'

            // Check document is not replicated in central because of the encryption error
            thread::sleep(Duration::from_secs(5));
            assert!(central_db.get_document("foo").is_err());
        },
    );
}

#[test]
fn decryption_error() {
    let config1 = utils::ReplicationTestConfiguration::default();
    let config2 = utils::ReplicationTestConfiguration {
        continuous: false,
        ..Default::default()
    };

    let context1 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: Some(encryptor),
        property_decryptor: Some(decryptor),
    };

    let context2 = ReplicationConfigurationContext {
        push_filter: None,
        pull_filter: None,
        conflict_resolver: None,
        property_encryptor: Some(encryptor),
        property_decryptor: Some(decryptor_err),
    };

    utils::with_three_dbs(
        config1,
        config2,
        Box::new(context1),
        Box::new(context2),
        |local_db1, local_db2, central_db, _repl1, repl2| {
            // Save doc 'foo' with an encryptable property
            {
                let mut doc = Document::new_with_id("foo");
                let mut props = doc.mutable_properties();
                props.at("i").put_i64(1234);
                props
                    .at("s")
                    .put_encrypt(&Encryptable::create_with_string("test_encryption"));
                local_db1
                    .save_document_with_concurency_control(
                        &mut doc,
                        ConcurrencyControl::FailOnConflict,
                    )
                    .expect("save");
            }

            // Check document is replicated in central
            assert!(utils::check_callback_with_wait(
                || central_db.get_document("foo").is_ok(),
                None
            ));
            {
                let doc_central = central_db.get_document("foo").unwrap();
                let dict = doc_central.properties();
                assert!(dict.to_keys_hash_set().get("encrypted$s").is_some());
            }

            // Manually trigger the replication
            repl2.start(false);

            // Check document is not replicated in DB2 because of the decryption error
            thread::sleep(Duration::from_secs(5));
            assert!(local_db2.get_document("foo").is_err());

            // Manually trigger the replication
            repl2.start(true); // 'reset_checkpoint = true' will trigger a new decryption, else the document will not be pulled again

            // Check document is not replicated in DB2 because of the decryption error
            thread::sleep(Duration::from_secs(5));
            assert!(local_db2.get_document("foo").is_err());
        },
    );
}

#[cfg(feature = "unsafe-threads-test")]
mod unsafe_test {
    use super::*;

    #[test]
    fn continuous() {
        let config1 = utils::ReplicationTestConfiguration {
            continuous: false,
            ..Default::default()
        };
        let config2: utils::ReplicationTestConfiguration = Default::default();

        utils::with_three_dbs(
            config1,
            config2,
            |local_db1, _local_db2, central_db, repl1, _repl2| {
                // Save doc
                utils::add_doc(local_db1, "foo", 1234, "Hello World!");

                // Check the replication process is not running automatically
                assert!(!utils::check_callback_with_wait(
                    || central_db.get_document("foo").is_ok(),
                    None
                ));

                // Manually trigger the replication
                repl1.start(false);

                // Check the replication was successful
                assert!(utils::check_callback_with_wait(
                    || central_db.get_document("foo").is_ok(),
                    None
                ));
            },
        );
    }
}
