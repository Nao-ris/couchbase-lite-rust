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

pub mod utils;

use std::sync::{Arc, Mutex};

//////// TESTS:

#[test]
fn db_properties() {
    utils::with_db(|db| {
        assert_eq!(db.name(), utils::DB_NAME);
        assert_eq!(db.count(), 0);
    });
}

#[test]
fn in_transaction() {
    utils::with_db(|db| {
        let result = db.in_transaction(|db| {
            let mut doc = Document::new_with_id("document");
            db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();
            Ok("document".to_string())
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "document");

        let result = db.in_transaction(|db| -> Result<String> {
            let mut doc = Document::new_with_id("document_error");
            db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();
            Err(couchbase_lite::Error::default())
        });

        assert!(result.is_err());

        assert!(db.get_document("document").is_ok());
        assert!(db.get_document("document_error").is_err());
    });
}

lazy_static! {
    static ref BUFFER_NOTIFICATIONS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref DOCUMENT_DETECTED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[test]
fn add_listener() {
    utils::set_static(&DOCUMENT_DETECTED, false);

    utils::with_db(|db| {
        let listener_token = db.add_listener(| _, doc_ids| {
            if doc_ids.first().unwrap() == "document" {
                utils::set_static(&DOCUMENT_DETECTED, true);
            }
        });

        let mut doc = Document::new_with_id("document");
        db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();

        assert!(utils::check_static_with_wait(&DOCUMENT_DETECTED, true, None));

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

        let listener_token = db.add_listener(| _, doc_ids| {
            if doc_ids.first().unwrap() == "document" {
                utils::set_static(&DOCUMENT_DETECTED, true);
            }
        });

        let mut doc = Document::new_with_id("document");
        db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();

        assert!(!utils::check_static_with_wait(&DOCUMENT_DETECTED, true, None));
        assert!(utils::check_static_with_wait(&BUFFER_NOTIFICATIONS, true, None));

        db.send_notifications();

        assert!(utils::check_static_with_wait(&DOCUMENT_DETECTED, true, None));

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
