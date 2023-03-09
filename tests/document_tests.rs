extern crate core;
extern crate couchbase_lite;

use self::couchbase_lite::*;
use std::time::Duration;

pub mod utils;

#[test]
fn document_new() {
    let document = Document::new();
    assert_ne!(document.id(), "");
    assert_eq!(document.revision_id(), None);
    assert_eq!(document.sequence(), 0);
    assert!(document.properties());
    assert_eq!(document.properties().count(), 0);
}

#[test]
fn document_new_with_id() {
    let document = Document::new_with_id("foo");
    assert_eq!(document.id(), "foo");
    assert_eq!(document.revision_id(), None);
    assert_eq!(document.sequence(), 0);
    assert!(document.properties());
    assert_eq!(document.properties().count(), 0);
}

#[test]
fn document_revision_id() {
    utils::with_db(|db| {
        let mut document = Document::new();
        assert_eq!(document.revision_id(), None);

        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        assert!(document.revision_id().is_some());

        let first_revision_id = String::from(document.revision_id().unwrap());
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        assert!(document.revision_id().is_some());
        let second_revision_id = String::from(document.revision_id().unwrap());
        assert_ne!(second_revision_id, first_revision_id);
    });
}

#[test]
fn document_sequence() {
    utils::with_db(|db| {
        let mut document_1 = Document::new();
        let mut document_2 = Document::new();
        assert_eq!(document_1.sequence(), 0);
        assert_eq!(document_2.sequence(), 0);

        db.save_document_with_concurency_control(
            &mut document_1,
            ConcurrencyControl::FailOnConflict,
        )
        .expect("save_document");
        db.save_document_with_concurency_control(
            &mut document_2,
            ConcurrencyControl::FailOnConflict,
        )
        .expect("save_document");
        assert_eq!(document_1.sequence(), 1);
        assert_eq!(document_2.sequence(), 2);
    });
}

#[test]
fn document_properties() {
    let mut document = Document::new();
    let mut properties = MutableDict::new();
    properties.at("foo").put_bool(false);
    properties.at("bar").put_bool(true);
    document.set_properties(&properties);
    let mut properties = document.mutable_properties();
    properties.at("baz").put_bool(true);
    properties.at("foo").put_bool(true);
    let properties = document.properties();
    assert_eq!(properties.count(), 3);
    assert_eq!(properties.get("foo").as_bool_or_false(), true);
    assert_eq!(properties.get("bar").as_bool_or_false(), true);
    assert_eq!(properties.get("baz").as_bool_or_false(), true);
}

#[test]
fn document_properties_as_json() {
    let mut document = Document::new();
    document
        .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
        .expect("set_properties_as_json");
    let properties = document.properties();
    assert_eq!(properties.count(), 2);
    assert_eq!(properties.get("foo").as_bool_or_false(), true);
    assert_eq!(properties.get("bar").as_bool_or_false(), true);
    let properties_as_json = document.properties_as_json();
    assert!(properties_as_json.contains(r#""foo":true"#));
    assert!(properties_as_json.contains(r#""bar":true"#));
}

#[test]
fn database_get_document() {
    utils::with_db(|db| {
        let mut document = Document::new_with_id("foo");
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        let document = db.get_document(document.id());
        assert!(document.is_ok());
        assert_eq!(document.unwrap().id(), "foo");
        let document = db.get_document("");
        assert!(document.is_err());
    });
}

#[test]
fn database_save_document() {
    utils::with_db(|db| {
        let mut document = Document::new_with_id("foo");
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        let mut document = db.get_document("foo").expect("get_document");
        {
            let mut document = db.get_document("foo").expect("get_document");
            document.mutable_properties().at("foo").put_i64(1);
            db.save_document_with_concurency_control(
                &mut document,
                ConcurrencyControl::FailOnConflict,
            )
            .expect("save_document");
        }
        document.mutable_properties().at("foo").put_i64(2);
        let conflict_error = db.save_document_with_concurency_control(
            &mut document,
            ConcurrencyControl::FailOnConflict,
        );
        assert!(conflict_error.is_err());
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::LastWriteWins)
            .expect("save_document");
        let document = db.get_document("foo").expect("get_document");
        assert_eq!(document.properties().get("foo").as_i64_or_0(), 2);
    });
}

#[test]
fn database_save_document_resolving() {
    utils::with_db(|db| {
        let mut document = Document::new_with_id("foo");
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        {
            let mut document = db.get_document("foo").unwrap();
            document.mutable_properties().at("foo").put_i64(1);
            db.save_document_with_concurency_control(
                &mut document,
                ConcurrencyControl::FailOnConflict,
            )
            .expect("save_document");
        }
        document.mutable_properties().at("foo").put_i64(2);
        document = db
            .save_document_resolving(&mut document, |document_a, document_b| {
                let property_a = document_a.properties().get("foo").as_i64_or_0();
                let property_b = document_b.properties().get("foo").as_i64_or_0();
                document_a
                    .mutable_properties()
                    .at("foo")
                    .put_i64(property_a + property_b);
                true
            })
            .expect("save_document_resolving");
        assert_eq!(document.properties().get("foo").as_i64_or_0(), 3);
        document = db.get_document("foo").unwrap();
        assert_eq!(document.properties().get("foo").as_i64_or_0(), 3);
    });
}

#[test]
fn database_purge_document() {
    utils::with_db(|db| {
        let mut document = Document::new();
        {
            db.save_document_with_concurency_control(
                &mut document,
                ConcurrencyControl::FailOnConflict,
            )
            .expect("save_document");
            let mut document = Document::new_with_id("foo");
            db.save_document_with_concurency_control(
                &mut document,
                ConcurrencyControl::FailOnConflict,
            )
            .expect("save_document");
        }
        db.purge_document(&document).expect("purge_document");
        db.purge_document_by_id("foo")
            .expect("purge_document_by_id");
        let document = db.get_document(document.id());
        assert!(document.is_err());
        let document = db.get_document("foo");
        assert!(document.is_err());
    });
}

#[test]
fn database_add_document_change_listener() {
    utils::with_db(|db| {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut document = Document::new_with_id("foo");
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        let listener_token = db.add_document_change_listener(
            &document,
            Box::new(move |_, document_id| {
                if let Some(id) = document_id {
                    assert_eq!(id, "foo");
                    sender.send(true).unwrap();
                }
            }),
        );
        document.mutable_properties().at("foo").put_i64(1);
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");

        receiver.recv_timeout(Duration::from_secs(1)).unwrap();

        let mut document = Document::new_with_id("bar");
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        assert!(receiver.recv_timeout(Duration::from_secs(10)).is_err());
        drop(listener_token);
    });
}

#[test]
fn database_delete_document() {
    let (sender, receiver) = std::sync::mpsc::channel();

    let context = ReplicationConfigurationContext {
        push_filter: Some(Box::new(move |document, is_deleted, _is_access_removed| {
            if is_deleted && document.id() == "foo" {
                sender.send(true).unwrap();
            }
            true
        })),
        ..Default::default()
    };

    let mut tester = utils::ReplicationTwoDbsTester::new(
        utils::ReplicationTestConfiguration::default(),
        Box::new(context),
    );

    tester.test(|local_db, central_db, _| {
        // Save doc 'foo'
        utils::add_doc(local_db, "foo", 1234, "Hello World!");

        // Check 'foo' is replicated to central and DB 2
        assert!(utils::check_callback_with_wait(
            || central_db.get_document("foo").is_ok(),
            None
        ));

        // Delete document in DB 1
        let document = local_db.get_document("foo").unwrap();
        local_db
            .delete_document_with_concurency_control(&document, ConcurrencyControl::FailOnConflict)
            .expect("delete_document");

        let document = local_db.get_document("foo").unwrap();
        assert!(document.is_deleted());

        // Check document is replicated with deleted flag
        receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    });
}

#[test]
fn database_document_expiration() {
    utils::with_db(|db| {
        let mut document = Document::new_with_id("foo");
        db.save_document_with_concurency_control(&mut document, ConcurrencyControl::FailOnConflict)
            .expect("save_document");
        let expiration = db.document_expiration("foo").expect("document_expiration");
        assert!(expiration.is_none());
        db.set_document_expiration("foo", Some(Timestamp(1000000000)))
            .expect("set_document_expiration");
        let expiration = db.document_expiration("foo").expect("document_expiration");
        assert!(expiration.is_some());
        assert_eq!(expiration.unwrap().0, 1000000000);
    });
}
