extern crate couchbase_lite;
use self::couchbase_lite::*;

pub mod utils;

use collection::DEFAULT_NAME;

#[test]
fn create_delete_scopes_collections() {
    utils::with_db(|db| {
        let unknown = "unknwon".to_string();
        let new_scope = "new_scope".to_string();
        let new_collection_1 = "new_collection_1".to_string();
        let new_collection_2 = "new_collection_2".to_string();

        // List scopes & collections
        assert_eq!(db.scope_names().unwrap(), vec![DEFAULT_NAME]);
        assert_eq!(
            db.collection_names(DEFAULT_NAME.to_string()).unwrap(),
            vec![DEFAULT_NAME]
        );

        assert_eq!(
            db.collection_names(unknown.clone()).unwrap(),
            Vec::<String>::default()
        );

        // Get scope
        assert!(db.scope(DEFAULT_NAME.to_string()).unwrap().is_some());
        assert!(db.scope(unknown.clone()).unwrap().is_none());
        assert_eq!(
            db.default_scope().unwrap(),
            db.scope(DEFAULT_NAME.to_string()).unwrap().unwrap()
        );

        // Get collection
        assert!(db
            .collection(DEFAULT_NAME.to_string(), DEFAULT_NAME.to_string())
            .unwrap()
            .is_some());
        assert!(db
            .collection(unknown.clone(), DEFAULT_NAME.to_string())
            .unwrap()
            .is_none()); // Invalid collection => None
        assert!(db
            .collection(DEFAULT_NAME.to_string(), unknown.clone())
            .is_err()); // Invalid scope => Err
        assert_eq!(
            db.default_collection().unwrap().unwrap(),
            db.collection(DEFAULT_NAME.to_string(), DEFAULT_NAME.to_string())
                .unwrap()
                .unwrap()
        );

        // Add collection in default scope
        {
            let c1_default_scope = db
                .create_collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
                .unwrap();

            assert_eq!(
                db.collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
                    .unwrap()
                    .unwrap(),
                c1_default_scope
            );
            assert_eq!(
                db.create_collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
                    .unwrap(),
                c1_default_scope
            );

            assert_eq!(
                db.collection_names(DEFAULT_NAME.to_string()).unwrap(),
                vec![DEFAULT_NAME.to_string(), new_collection_1.clone()]
            );
        }

        // Add collection in new scope
        {
            let c2_new_scope = db
                .create_collection(new_collection_2.clone(), new_scope.clone())
                .unwrap();

            assert_eq!(
                db.collection(new_collection_2.clone(), new_scope.clone())
                    .unwrap()
                    .unwrap(),
                c2_new_scope
            );
            assert_eq!(
                db.create_collection(new_collection_2.clone(), new_scope.clone())
                    .unwrap(),
                c2_new_scope
            );

            assert_eq!(
                db.scope_names().unwrap(),
                vec![DEFAULT_NAME.to_string(), new_scope.clone()]
            );
            assert_eq!(
                db.collection_names(new_scope.clone()).unwrap(),
                vec![new_collection_2.clone()]
            );
        }

        // Delete collections
        assert!(db
            .delete_collection(DEFAULT_NAME.to_string(), unknown.clone())
            .is_err()); // Invalid scope => Err
        assert!(db
            .delete_collection(unknown.clone(), DEFAULT_NAME.to_string())
            .is_ok()); // Invalid collection => Ok
        assert!(db
            .delete_collection(unknown.clone(), unknown.clone())
            .is_ok()); // Invalid collection & scope => Ok

        assert!(db
            .delete_collection(new_collection_2.clone(), new_scope.clone())
            .is_ok());
        assert!(db.scope(new_scope.clone()).unwrap().is_none());
        assert_eq!(db.scope_names().unwrap(), vec![DEFAULT_NAME]);
        assert!(db
            .collection(new_collection_2.clone(), new_scope.clone())
            .unwrap()
            .is_none());
        assert_eq!(
            db.collection_names(new_scope.clone()).unwrap(),
            Vec::<String>::default()
        );

        assert!(db
            .delete_collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
            .is_ok());
        assert!(db.scope(DEFAULT_NAME.to_string()).unwrap().is_some()); // Default scope kept
        assert!(db
            .collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
            .unwrap()
            .is_none());

        assert!(db
            .delete_collection(DEFAULT_NAME.to_string(), DEFAULT_NAME.to_string())
            .is_err()); // Impossible to delete default collection
    });
}

#[test]
fn collections_accessors() {
    utils::with_db(|db| {
        // Setup
        let new_scope = "new_scope".to_string();
        let new_collection_1 = "new_collection_1".to_string();
        let new_collection_2 = "new_collection_2".to_string();

        let c1_default_scope = db
            .create_collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
            .unwrap();
        assert_eq!(
            db.collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
                .unwrap()
                .unwrap(),
            c1_default_scope
        );

        let c2_new_scope = db
            .create_collection(new_collection_2.clone(), new_scope.clone())
            .unwrap();
        assert_eq!(
            db.collection(new_collection_2.clone(), new_scope.clone())
                .unwrap()
                .unwrap(),
            c2_new_scope
        );

        let c1_new_scope = db
            .create_collection(new_collection_1.clone(), new_scope.clone())
            .unwrap();
        assert_eq!(
            db.collection(new_collection_1.clone(), new_scope.clone())
                .unwrap()
                .unwrap(),
            c1_new_scope
        );

        let default_scope = db.scope(DEFAULT_NAME.to_string()).unwrap().unwrap();
        let new_actual_scope = db.scope(new_scope.clone()).unwrap().unwrap();

        // Scope
        assert_eq!(c1_default_scope.scope(), default_scope);
        assert_eq!(c2_new_scope.scope(), new_actual_scope);
        assert_eq!(c1_new_scope.scope(), new_actual_scope);

        // Name
        assert_eq!(c1_default_scope.name(), new_collection_1.clone());
        assert_eq!(c2_new_scope.name(), new_collection_2.clone());
        assert_eq!(c1_new_scope.name(), new_collection_1.clone());

        // Count
        assert_eq!(c1_default_scope.count(), 0);
        assert_eq!(c2_new_scope.count(), 0);
        assert_eq!(c1_new_scope.count(), 0);
    });
}

#[test]
fn scope_accessors() {
    utils::with_db(|db| {
        // Setup
        let new_scope = "new_scope".to_string();
        let new_collection_1 = "new_collection_1".to_string();
        let new_collection_2 = "new_collection_2".to_string();

        let c1_default_scope = db
            .create_collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
            .unwrap();
        assert_eq!(
            db.collection(new_collection_1.clone(), DEFAULT_NAME.to_string())
                .unwrap()
                .unwrap(),
            c1_default_scope
        );

        let c2_new_scope = db
            .create_collection(new_collection_2.clone(), new_scope.clone())
            .unwrap();
        assert_eq!(
            db.collection(new_collection_2.clone(), new_scope.clone())
                .unwrap()
                .unwrap(),
            c2_new_scope
        );

        let c1_new_scope = db
            .create_collection(new_collection_1.clone(), new_scope.clone())
            .unwrap();
        assert_eq!(
            db.collection(new_collection_1.clone(), new_scope.clone())
                .unwrap()
                .unwrap(),
            c1_new_scope
        );

        let default_scope = db.scope(DEFAULT_NAME.to_string()).unwrap().unwrap();
        let new_actual_scope = db.scope(new_scope.clone()).unwrap().unwrap();

        // Name
        assert_eq!(default_scope.name(), DEFAULT_NAME.to_string());
        assert_eq!(new_actual_scope.name(), new_scope.clone());

        // Collections
        assert_eq!(
            default_scope.collection_names().unwrap(),
            vec![DEFAULT_NAME.to_string(), new_collection_1.clone()]
        );
        assert_eq!(
            new_actual_scope.collection_names().unwrap(),
            vec![new_collection_2.clone(), new_collection_1.clone()]
        );

        assert!(default_scope
            .collection("unknwon".to_string())
            .unwrap()
            .is_none());

        assert_eq!(
            default_scope
                .collection(new_collection_1.clone())
                .unwrap()
                .unwrap(),
            c1_default_scope
        );
        assert_eq!(
            new_actual_scope
                .collection(new_collection_2.clone())
                .unwrap()
                .unwrap(),
            c2_new_scope
        );
        assert_eq!(
            new_actual_scope
                .collection(new_collection_1.clone())
                .unwrap()
                .unwrap(),
            c1_new_scope
        );
    });
}

#[test]
fn collection_documents() {
    utils::with_db(|db| {
        // Collection 1
        let mut collection_1 = db
            .create_collection("collection_1".to_string(), DEFAULT_NAME.to_string())
            .unwrap();
        let mut doc_1 = Document::new_with_id("foo");
        doc_1
            .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
            .expect("set_properties_as_json");
        collection_1
            .save_document(&mut doc_1)
            .expect("save_document");

        assert!(collection_1.get_document("foo").is_ok());
        assert!(db
            .default_collection()
            .unwrap()
            .unwrap()
            .get_document("foo")
            .is_err()); // Document 1 not in default collection

        // Collection 2
        let mut collection_2 = db
            .create_collection("collection_2".to_string(), "scope_1".to_string())
            .unwrap();
        assert!(collection_2.get_document("foo").is_err()); // Document 1 not in collection 2

        let mut doc_2 = Document::new_with_id("foo");
        doc_2
            .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
            .expect("set_properties_as_json");
        collection_2
            .save_document(&mut doc_2)
            .expect("save_document");

        assert!(collection_2.get_document("foo").is_ok());

        // Collection 3
        let mut collection_3 = db
            .create_collection("collection_3".to_string(), "scope_1".to_string())
            .unwrap();
        assert!(collection_3.get_document("foo").is_err()); // Document 2 not in collection 3 even though collections 2 & 3 are in the same scope

        let mut doc_3 = Document::new_with_id("foo");
        doc_3
            .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
            .expect("set_properties_as_json");
        collection_3
            .save_document(&mut doc_3)
            .expect("save_document");

        assert!(collection_3.get_document("foo").is_ok());

        // Delete documents
        assert!(collection_1.delete_document(&doc_1).is_ok());
        assert!(collection_1.get_document("foo").unwrap().is_deleted());

        assert!(collection_2.delete_document(&doc_2).is_ok());
        assert!(collection_2.get_document("foo").unwrap().is_deleted());

        assert!(collection_3.delete_document(&doc_3).is_ok());
        assert!(collection_3.get_document("foo").unwrap().is_deleted());
    });
}

#[test]
fn queries() {
    utils::with_db(|db| {
        // Setup
        {
            let mut default_collection = db.default_collection().unwrap().unwrap();
            let mut doc_1 = Document::new_with_id("foo");
            doc_1
                .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
                .expect("set_properties_as_json");
            default_collection
                .save_document(&mut doc_1)
                .expect("save_document");

            let mut collection_1 = db
                .create_collection("collection_1".to_string(), DEFAULT_NAME.to_string())
                .unwrap();
            let mut doc_1 = Document::new_with_id("foo1");
            doc_1
                .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
                .expect("set_properties_as_json");
            collection_1
                .save_document(&mut doc_1)
                .expect("save_document");

            let mut collection_2 = db
                .create_collection("collection_2".to_string(), "scope_1".to_string())
                .unwrap();
            let mut doc_2 = Document::new_with_id("foo2");
            doc_2
                .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
                .expect("set_properties_as_json");
            collection_2
                .save_document(&mut doc_2)
                .expect("save_document");

            let mut collection_3 = db
                .create_collection("collection_3".to_string(), "scope_1".to_string())
                .unwrap();
            let mut doc_3 = Document::new_with_id("foo3");
            doc_3
                .set_properties_as_json(r#"{"foo":true,"bar":true}"#)
                .expect("set_properties_as_json");
            collection_3
                .save_document(&mut doc_3)
                .expect("save_document");
        }

        fn query(db: &mut Database, query: &str) -> Vec<String> {
            let query = Query::new(db, QueryLanguage::N1QL, query).expect("create query");

            let mut query_result = query.execute().expect("execute");

            let mut results = vec![];
            while let Some(row) = query_result.next() {
                results.push(row.get(0).as_string().unwrap_or("").to_string());
            }
            results
        }

        assert_eq!(query(db, "SELECT _id FROM _"), vec!["foo".to_string()]); // Default collection
        assert_eq!(
            query(db, "SELECT _id FROM collection_1"),
            vec!["foo1".to_string()]
        ); // Collection must be in default scope with this query
        assert_eq!(
            query(db, "SELECT _id FROM _default.collection_1"),
            vec!["foo1".to_string()]
        );
        assert_eq!(
            query(db, "SELECT _id FROM scope_1.collection_2"),
            vec!["foo2".to_string()]
        );
        assert_eq!(
            query(db, "SELECT _id FROM scope_1.collection_3"),
            vec!["foo3".to_string()]
        );
    });
}
