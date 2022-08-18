extern crate couchbase_lite;

use couchbase_lite::index::ValueIndexConfiguration;

use self::couchbase_lite::*;

pub mod utils;

#[test]
fn query() {
    utils::with_db(|db| {
        utils::add_doc(db, "doc-1", 1, "one");
        utils::add_doc(db, "doc-2", 2, "two");
        utils::add_doc(db, "doc-3", 3, "three");

        let query = Query::new(
            db,
            QueryLanguage::N1QL,
            "select i, s from _ where i > 1 order by i",
        )
        .expect("create query");
        assert_eq!(query.column_count(), 2);
        assert_eq!(query.column_name(0), Some("i"));
        assert_eq!(query.column_name(1), Some("s"));

        // Step through the iterator manually:
        let mut results = query.execute().expect("execute");
        let mut row = results.next().unwrap(); //FIXME: Do something about the (&results). requirement
        let mut i = row.get(0);
        let mut s = row.get(1);
        assert_eq!(i.as_i64().unwrap(), 2);
        assert_eq!(s.as_string().unwrap(), "two");
        assert_eq!(row.as_dict().to_json(), r#"{"i":2,"s":"two"}"#);

        row = results.next().unwrap();
        i = row.get(0);
        s = row.get(1);
        assert_eq!(i.as_i64().unwrap(), 3);
        assert_eq!(s.as_string().unwrap(), "three");
        assert_eq!(row.as_dict().to_json(), r#"{"i":3,"s":"three"}"#);

        assert!(results.next().is_none());

        // Now try a for...in loop:
        let mut n = 0;
        for row in query.execute().expect("execute") {
            match n {
                0 => {
                    assert_eq!(row.as_array().to_json(), r#"[2,"two"]"#);
                    assert_eq!(row.as_dict().to_json(), r#"{"i":2,"s":"two"}"#);
                }
                1 => {
                    assert_eq!(row.as_array().to_json(), r#"[3,"three"]"#);
                    assert_eq!(row.as_dict().to_json(), r#"{"i":3,"s":"three"}"#);
                }
                _ => {
                    panic!("Too many rows ({})", n);
                }
            }
            n += 1;
        }
        assert_eq!(n, 2);
    });
}

#[test]
fn array_contains_buggy() {
    utils::with_db(|db| {
        let mut doc = Document::new();
        doc.set_properties_as_json("{\"id\":\"array_doc\",\"array\":[\"id1\",\"id2\",\"id3\"]}")
            .unwrap();
        db.save_document(&mut doc).expect("save");

        let mut doc = Document::new();
        doc.set_properties_as_json("{\"id\":\"id2\"}").unwrap();
        db.save_document(&mut doc).expect("save");

        // First query, should return 'array_doc'
        let query = Query::new(
            db,
            QueryLanguage::N1QL,
            "select r.id \
                from _ r \
                join _ f ON \
                ARRAY_CONTAINS(r.array, f.id)",
        )
        .expect("create query");

        let mut results = query.execute().expect("execute");
        // CBL Query: {Query#3} Compiling N1QL query: select r.id from _ r join _ f ON ARRAY_CONTAINS(r.array, f.id)
        // CBL Query: {Query#3} Compiled as SELECT fl_result(fl_value(r.body, 'id')) FROM kv_default AS r INNER JOIN kv_default AS f ON (array_contains(fl_value(r.body, 'array'), fl_value(f.body, 'id'))) AND (f.flags & 1 = 0) WHERE (r.flags & 1 = 0)

        let row = results.next().unwrap();
        assert_eq!(row.get(0).as_string().unwrap(), "array_doc");
        assert!(results.next().is_none());

        // First query, should also return 'array_doc' but is not returning anything because of the alias
        let query = Query::new(
            db,
            QueryLanguage::N1QL,
            "select r.id as id \
                from _ r \
                join _ f ON \
                ARRAY_CONTAINS(r.array, f.id)",
        )
        .expect("create query");

        let mut results = query.execute().expect("execute");
        // CBL Query: {Query#5} Compiling N1QL query: select r.id as id from _ r join _ f ON ARRAY_CONTAINS(r.array, f.id)
        // CBL Query: {Query#5} Compiled as SELECT fl_result(fl_value(r.body, 'id')) AS id FROM kv_default AS r INNER JOIN kv_default AS f ON (array_contains(fl_value(r.body, 'array'), id)) AND (f.flags & 1 = 0) WHERE (r.flags & 1 = 0)

        assert!(results.next().is_none());
    });
}

#[test]
fn indexes() {
    utils::with_db(|db| {
        assert!(db
            .create_index(
                "new_index",
                &ValueIndexConfiguration::new(QueryLanguage::JSON, r#"[[".id"]]"#),
            )
            .unwrap());

        let value = db.get_index_names().iter().next().unwrap();
        let name = value.as_string().unwrap();
        assert_eq!(name, "new_index");

        db.delete_index("idx").unwrap();
        assert_eq!(db.get_index_names().count(), 1);

        db.delete_index("new_index").unwrap();
        assert_eq!(db.get_index_names().count(), 0);
    });
}
