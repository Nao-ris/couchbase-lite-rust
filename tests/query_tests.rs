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
