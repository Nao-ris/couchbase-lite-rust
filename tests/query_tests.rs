extern crate couchbase_lite;

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
        let results = query.execute().expect("execute");
        let mut row = (&results).next().unwrap(); //FIXME: Do something about the (&results). requirement
        let mut i = row.get(0).as_i64().unwrap();
        let mut s = row.get(1).as_string().unwrap();
        assert_eq!(i, 2);
        assert_eq!(s, "two");
        assert_eq!(row.as_dict().to_json(), r#"{"i":2,"s":"two"}"#);

        row = (&results).next().unwrap();
        i = row.get(0).as_i64().unwrap();
        s = row.get(1).as_string().unwrap();
        assert_eq!(i, 3);
        assert_eq!(s, "three");
        assert_eq!(row.as_dict().to_json(), r#"{"i":3,"s":"three"}"#);

        assert!((&results).next().is_none());

        // Now try a for...in loop:
        let mut n = 0;
        for row in &query.execute().expect("execute") {
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
