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

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

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
    let path = Path::new("db");
    let (db_thread, db_exec) = utils::run_db_thread(path);

    db_exec.spawn(move |db| {
        if let Some(db) = db.as_mut() {
            let result = db.in_transaction(transaction_callback);

            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "document");

            let result = db.in_transaction(transaction_callback_error);

            assert!(result.is_err());

            assert!(db.get_document("document").is_ok());
            assert!(db.get_document("document_error").is_err());
        }
    });

    utils::close_db(db_thread, db_exec);
    utils::delete_db(path);
}

fn transaction_callback(db: &mut Database) -> Result<String> {
    let mut doc = Document::new_with_id("document");
    db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();
    Ok("document".to_string())
}

fn transaction_callback_error(db: &mut Database) -> Result<String> {
    let mut doc = Document::new_with_id("document_error");
    db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();
    Err(couchbase_lite::Error::default())
}

lazy_static! {
    static ref BUFFER_NOTIFICATIONS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref DOCUMENT_DETECTED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[test]
fn add_listener() {
    utils::set_static(&DOCUMENT_DETECTED, false);

    let path = Path::new("db");

    let (db_thread, db_exec) = utils::run_db_thread(path);

    db_exec.spawn(move |db| {
        if let Some(db) = db.as_mut() {
            let listener_token = db.add_listener(| _, doc_ids| {
                if doc_ids.first().unwrap() == "document" {
                    utils::set_static(&DOCUMENT_DETECTED, true);
                }
            });

            let mut doc = Document::new_with_id("document");
            db.save_document(&mut doc, ConcurrencyControl::LastWriteWins).unwrap();

            assert!(utils::check_static_with_wait(&DOCUMENT_DETECTED));

            drop(listener_token);
        } else {
            println!("Error: DB is NOT open");
        }
    });

    utils::close_db(db_thread, db_exec);
    utils::delete_db(path);
}

#[test]
fn buffer_notifications() {
    utils::set_static(&BUFFER_NOTIFICATIONS, false);
    utils::set_static(&DOCUMENT_DETECTED, false);

    let path = Path::new("db");

    let (db_thread, db_exec) = utils::run_db_thread(path);

    db_exec.spawn(move |db| {
        if let Some(db) = db.as_mut() {
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

            assert!(!utils::check_static_with_wait(&DOCUMENT_DETECTED));
            assert!(utils::check_static_with_wait(&BUFFER_NOTIFICATIONS));

            db.send_notifications();

            assert!(utils::check_static_with_wait(&DOCUMENT_DETECTED));

            drop(listener_token);
        } else {
            println!("Error: DB is NOT open");
        }
    });

    utils::close_db(db_thread, db_exec);
    utils::delete_db(path);
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
