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
extern crate tempdir;
extern crate lazy_static;

use self::couchbase_lite::*;
use self::tempdir::TempDir;
use lazy_static::lazy_static;
use utils::init_logging;

pub mod utils;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

//////// TESTS:

// Only for buffer_notifications test
lazy_static! {
    static ref BUFFER_NOTIFICATIONS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref DOCUMENT_DETECTED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[test]
fn delete_file() {
    pub const DB_NAME: &str = "test_db";

    init_logging();

    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: None,
    };
    if let Err(_) = Database::open(DB_NAME, Some(cfg)) {
        panic!("The database could not be opened");
    }
    assert!(Database::exists(DB_NAME, tmp_dir.path()));

    Database::delete_file(DB_NAME, tmp_dir.path()).expect("Database deletion failed");
    assert!(!Database::exists(DB_NAME, tmp_dir.path()));
}

#[test]
fn copy_file() {
    pub const DB_NAME: &str = "test_db";
    pub const DB_NAME_BACKUP: &str = "test_db_backup";

    init_logging();

    // Initial DB
    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: None,
    };
    match Database::open(DB_NAME, Some(cfg.clone())) {
        Ok(mut db) => {
            let mut doc = Document::new_with_id("foo");

            let mut props = doc.mutable_properties();
            props.at("i").put_i64(1);
            props.at("s").put_string("test");

            db.save_document_with_concurency_control(&mut doc, ConcurrencyControl::FailOnConflict)
                .expect("save");
        }
        Err(_) => panic!("The initial database could not be opened"),
    }
    assert!(Database::exists(DB_NAME, tmp_dir.path()));

    // Copy DB
    let current_path = tmp_dir.path().join(format!("{DB_NAME}.cblite2"));
    Database::copy_file(current_path.as_path(), DB_NAME_BACKUP, Some(cfg.clone()))
        .expect("Database copy failed");

    assert!(Database::exists(DB_NAME, tmp_dir.path()));
    assert!(Database::exists(DB_NAME_BACKUP, tmp_dir.path()));

    // Check document is inside the new DB
    match Database::open(DB_NAME_BACKUP, Some(cfg)) {
        Ok(db) => {
            let doc = db.get_document("foo").unwrap();

            assert_eq!(doc.properties().get("i").as_i64().unwrap(), 1);
            assert_eq!(doc.properties().get("s").as_string().unwrap(), "test");
        }
        Err(_) => panic!("The new database could not be opened"),
    }
}

#[test]
fn in_transaction() {
    utils::with_db(|db| {
        let result = db.in_transaction(|db| {
            let mut doc = Document::new_with_id("document");
            db.save_document_with_concurency_control(&mut doc, ConcurrencyControl::LastWriteWins)
                .unwrap();
            Ok("document".to_string())
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "document");

        let result = db.in_transaction(|db| -> Result<String> {
            let mut doc = Document::new_with_id("document_error");
            db.save_document_with_concurency_control(&mut doc, ConcurrencyControl::LastWriteWins)
                .unwrap();
            Err(couchbase_lite::Error::default())
        });

        assert!(result.is_err());

        assert!(db.get_document("document").is_ok());
        assert!(db.get_document("document_error").is_err());
    });
}

#[test]
fn db_properties() {
    utils::with_db(|db| {
        assert_eq!(db.name(), utils::DB_NAME);
        assert_eq!(db.count(), 0);
    });
}

#[test]
fn db_encryption_key() {
    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg_no_encryption = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: None,
    };
    let encryption_key = EncryptionKey::new_from_password("password1").unwrap();
    let cfg_encryption1 = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: Some(encryption_key.clone()),
    };

    // Create database with no encryption & one document
    {
        let mut db = Database::open(utils::DB_NAME, Some(cfg_no_encryption.clone())).unwrap();
        let mut doc = Document::new_with_id("foo");
        assert!(db
            .save_document_with_concurency_control(&mut doc, ConcurrencyControl::LastWriteWins)
            .is_ok());
    }

    // Assert database can only be opened with no ecryption & doc can be retrieved, then add encryption
    assert!(Database::open(utils::DB_NAME, Some(cfg_no_encryption.clone())).is_ok());
    assert!(Database::open(utils::DB_NAME, Some(cfg_encryption1.clone())).is_err());
    {
        let mut db = Database::open(utils::DB_NAME, Some(cfg_no_encryption.clone())).unwrap();
        assert!(db.get_document("foo").is_ok());
        assert!(db.change_encryption_key(&encryption_key).is_ok());
    }

    // Assert database can only be opened with ecryption & doc can be retrieved
    assert!(Database::open(utils::DB_NAME, Some(cfg_no_encryption.clone())).is_err());
    assert!(Database::open(utils::DB_NAME, Some(cfg_encryption1.clone())).is_ok());
    {
        let db = Database::open(utils::DB_NAME, Some(cfg_encryption1.clone())).unwrap();
        assert!(db.get_document("foo").is_ok());
    }
}

#[test]
fn add_listener() {
    utils::with_db(|db| {
        let (sender, receiver) = std::sync::mpsc::channel();
        let listener_token = db.add_listener(Box::new(move |_, doc_ids| {
            if doc_ids.first().unwrap() == "document" {
                sender.send(true).unwrap();
            }
        }));

        let mut doc = Document::new_with_id("document");
        db.save_document_with_concurency_control(&mut doc, ConcurrencyControl::LastWriteWins)
            .unwrap();

        receiver.recv_timeout(Duration::from_secs(1)).unwrap();

        drop(listener_token);
    });
}

#[test]
fn buffer_notifications() {
    utils::set_static(&BUFFER_NOTIFICATIONS, false);
    utils::set_static(&DOCUMENT_DETECTED, false);

    utils::with_db(|db| {
        db.buffer_notifications(|_| {
            utils::set_static(&BUFFER_NOTIFICATIONS, true);
        });

        let listener_token = db.add_listener(Box::new(move |_, doc_ids| {
            if doc_ids.first().unwrap() == "document" {
                utils::set_static(&DOCUMENT_DETECTED, true);
            }
        }));

        let mut doc = Document::new_with_id("document");
        db.save_document_with_concurency_control(&mut doc, ConcurrencyControl::LastWriteWins)
            .unwrap();

        assert!(!utils::check_static_with_wait(
            &DOCUMENT_DETECTED,
            true,
            None
        ));
        assert!(utils::check_static_with_wait(
            &BUFFER_NOTIFICATIONS,
            true,
            None
        ));

        db.send_notifications();

        assert!(utils::check_static_with_wait(
            &DOCUMENT_DETECTED,
            true,
            None
        ));

        drop(listener_token);
    });
}

/*
// This test doesn't and shouldn't compile -- it tests that the borrow-checker will correctly
// prevent Fleece data from being used after its document has been freed.
#[test]
fn document_borrow_check() {
    let mut db = Database::open(DB_NAME, None).expect("open db");
    let v : Value;
    {
        let doc = db.get_document("foo").expect("get doc");
        v = doc.properties().get("a");
    }
    println!("v = {:?}", v);
}
*/
