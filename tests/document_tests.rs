
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
    let mut initial_properties = MutableDict::new();
    initial_properties.at("foo").put_bool(false);
    initial_properties.at("bar").put_bool(true);
    document.set_properties(initial_properties);
    let mut set_properties = document.mutable_properties();
    set_properties.at("baz").put_bool(true);
    set_properties.at("foo").put_bool(true);
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
fn save_document() {
    utils::with_db(|db| {
        {
            let mut doc = Document::new_with_id("foo");
            let mut props = doc.mutable_properties();
            props.at("i").put_i64(1234);
            props.at("s").put_string("Hello World!");

            db.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");
        }
        {
            let doc = db.get_document("foo").expect("reload document");
            let props = doc.properties();
            verbose!("Blah blah blah");
            info!("Interesting: {} = {}", 2+2, 4);
            warn!("This is a warning");
            error!("Oh no, props = {}", props);
            assert_eq!(props.to_json(), r#"{"i":1234,"s":"Hello World!"}"#);
        }
    });
}