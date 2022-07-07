
extern crate couchbase_lite;
extern crate core;

use self::couchbase_lite::*;

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
        db.save_document(&mut document, ConcurrencyControl::FailOnConflict).expect("save_document");
        assert!(document.revision_id().is_some());
        let first_revision_id = String::from(document.revision_id().unwrap());
        db.save_document(&mut document, ConcurrencyControl::FailOnConflict).expect("save_document");
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
        db.save_document(&mut document_1, ConcurrencyControl::FailOnConflict).expect("save_document");
        db.save_document(&mut document_2, ConcurrencyControl::FailOnConflict).expect("save_document");
        assert_eq!(document_1.sequence(), 1);
        assert_eq!(document_2.sequence(), 2);
    });
}

#[test]
fn document_properties() {
    let mut document = Document::new();
    {
        let mut initial_properties = MutableDict::new();
        initial_properties.at("foo").put_bool(false);
        initial_properties.at("bar").put_bool(true);
        document.set_properties(initial_properties);
    }
    {
        let mut set_properties = document.mutable_properties();
        set_properties.at("baz").put_bool(true);
        set_properties.at("foo").put_bool(true);
    }
    let final_properties = document.properties();
    assert_eq!(final_properties.count(), 3);
    assert_eq!(final_properties.get("foo").as_bool_or_false(), true);
    assert_eq!(final_properties.get("bar").as_bool_or_false(), true);
    assert_eq!(final_properties.get("baz").as_bool_or_false(), true);
}

#[test]
fn document_properties_as_json() {
    let mut document = Document::new();
    document.set_properties_as_json(r#"{"foo":true,"bar":true}"#).expect("set_properties_as_json");
    let final_properties = document.properties();
    assert_eq!(final_properties.count(), 2);
    assert_eq!(final_properties.get("foo").as_bool_or_false(), true);
    assert_eq!(final_properties.get("bar").as_bool_or_false(), true);
    let properties_as_json = document.properties_as_json();
    assert!(properties_as_json.contains(r#""foo":true"#));
    assert!(properties_as_json.contains(r#""bar":true"#));
}

#[test]
fn database_get_document() {
    utils::with_db(|db| {
        let mut document = Document::new();
        db.save_document(&mut document, ConcurrencyControl::FailOnConflict).expect("save_document");
        let got_document = db.get_document(document.id());
        assert!(got_document.is_ok());
        assert_eq!(got_document.unwrap().id(), document.id());
        let no_document = db.get_document("");
        assert!(no_document.is_err());
    });
}

#[test]
fn database_save_document() {
    utils::with_db(|db| {
        {
            let mut document = Document::new_with_id("foo");
            db.save_document(&mut document, ConcurrencyControl::FailOnConflict).expect("save_document");
        }
        {
            let document = db.get_document("foo");
            assert!(document.is_ok());
        }
    });
}

#[test]
fn database_save_document_resolving() {
    utils::with_db(|_db| {
        // TODO
    });
}

#[test]
fn database_purge_document_by_id() {
    utils::with_db(|db| {
        {
            let mut document = Document::new_with_id("foo");
            db.save_document(&mut document, ConcurrencyControl::FailOnConflict).expect("save_document");
        }
        {
            db.purge_document_by_id("foo").expect("purge_document_by_id");
        }
        {
            let document = db.get_document("foo");
            assert!(document.is_err());
        }
    });
}

#[test]
fn database_document_expiration() {
    utils::with_db(|db| {
        {
            let mut document = Document::new_with_id("foo");
            db.save_document(&mut document, ConcurrencyControl::FailOnConflict).expect("save_document");
        }
        let initial_expiration = db.document_expiration("foo").expect("document_expiration");
        assert!(initial_expiration.is_none());
        db.set_document_expiration("foo", Some(Timestamp(1000000000))).expect("set_document_expiration");
        let set_expiration = db.document_expiration("foo").expect("document_expiration");
        assert!(set_expiration.is_some());
        assert_eq!(set_expiration.unwrap().0, 1000000000);
    });
}