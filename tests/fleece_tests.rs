// Unit tests for Fleece
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

#![cfg(test)]

extern crate couchbase_lite;

use couchbase_lite::*;

#[test]
fn empty_values() {
    let v = Value::default();
    assert_eq!(v.get_type(), ValueType::Undefined);
    assert!(!v.is_type(ValueType::Bool));
    assert!(!v.is_number());
    assert!(!v.is_integer());
    assert_eq!(v.as_i64(), None);
    assert!(!v.as_array());
    assert!(!v.as_dict());
    assert!(!v);
    assert_eq!(v, Value::UNDEFINED);
    assert!(v == v);
}

#[test]
fn basic_values() {
    let doc = Fleece::parse_json(r#"{"i":1234,"f":12.34,"a":[1, 2],"s":"Foo"}"#).unwrap();
    let dict = doc.as_dict();
    assert_eq!(dict.count(), 4);

    let i = dict.get("i");
    assert!(i.is_number());
    assert!(i.is_integer());
    assert_eq!(i.as_i64(), Some(1234));
    assert_eq!(i.as_i64_or_0(), 1234);
    assert_eq!(i.as_f64(), Some(1234.0));
    assert_eq!(i.as_string(), None);

    let f = dict.get("f");
    assert!(f.is_number());
    assert!(!f.is_integer());
    assert_eq!(f.as_i64(), None);
    assert_eq!(f.as_i64_or_0(), 12);
    assert_eq!(f.as_f64(), Some(12.34));
    assert_eq!(f.as_string(), None);

    assert_eq!(dict.get("j"), Value::UNDEFINED);

    assert_eq!(dict.get("s").as_string(), Some("Foo"));

    let a = dict.get("a").as_array();
    assert!(a);
    assert_eq!(a, a);
    assert_eq!(a.count(), 2);
    assert_eq!(a.get(0).as_i64(), Some(1));
    assert_eq!(a.get(1).as_i64(), Some(2));
    assert_eq!(a.get(2).as_i64(), None);

    assert_eq!(
        doc.root().to_json(),
        r#"{"a":[1,2],"f":12.34,"i":1234,"s":"Foo"}"#
    );
    assert_eq!(
        format!("{}", doc.root()),
        r#"{"a":[1,2],"f":12.34,"i":1234,"s":"Foo"}"#
    );
}

#[test]
fn nested_borrow_check() {
    let v: Value;
    let mut str = String::new();

    let doc = Fleece::parse_json(r#"{"i":1234,"f":12.34,"a":[1, 2],"s":"Foo"}"#).unwrap();
    {
        let dict = doc.as_dict();
        str.push_str(dict.get("s").as_string().unwrap());
        v = dict.get("a");
    }
    // It's OK that `dict` has gone out of scope, because `v`s scope is `doc`, not `dict`.
    println!("v = {:?}", v);
    println!("str = {}", str);
}

/*
// This test doesn't and shouldn't compile -- it tests that the borrow-checker will correctly
// prevent Fleece data from being used after its document has been freed.
#[test]
fn borrow_check() {
    let v : Value;
    let str : &str;
    {
        let doc = Fleece::parse_json(r#"{"i":1234,"f":12.34,"a":[1, 2],"s":"Foo"}"#).unwrap();
        let dict = doc.as_dict();
        v = dict.get("a");
        str = dict.get("s").as_string().unwrap();
    }
    println!("v = {:?}", v);
    println!("str = {}", str);
}
*/

#[test]
fn dict_to_hash_set() {
    let mut mut_dict = MutableDict::new();

    mut_dict.at("id1").put_bool(true);
    mut_dict.at("id2").put_bool(true);

    let dict = mut_dict.as_dict();

    let hash_set = dict.to_keys_hash_set();

    assert_eq!(hash_set.len(), 2);
    assert_eq!(hash_set.get("id1"), Some(&"id1".to_string()));
    assert_eq!(hash_set.get("id2"), Some(&"id2".to_string()));
}

#[test]
fn mutable_dict() {
    let mut dict = MutableDict::new();
    assert_eq!(dict.count(), 0);
    assert_eq!(dict.get("a"), Value::UNDEFINED);

    dict.at("i").put_i64(1234);
    dict.at("s").put_string("Hello World!");

    assert_eq!(format!("{}", dict), r#"{"i":1234,"s":"Hello World!"}"#);

    assert_eq!(dict.count(), 2);
    assert_eq!(dict.get("i").as_i64(), Some(1234));
    assert_eq!(dict.get("s").as_string(), Some("Hello World!"));
    assert!(!dict.get("?"));

    dict.remove("i");
    assert!(!dict.get("i"));
}

#[test]
fn mutable_dict_to_from_hash_map() {
    let mut dict = MutableDict::new();

    dict.at("id1").put_string("value1");
    dict.at("id2").put_string("value2");

    let hash_map = dict.to_hashmap();
    assert_eq!(hash_map.len(), 2);
    assert_eq!(hash_map.get("id1"), Some(&"value1".to_string()));
    assert_eq!(hash_map.get("id2"), Some(&"value2".to_string()));

    let new_dict = MutableDict::from_hashmap(&hash_map);
    assert_eq!(new_dict.count(), 2);
    assert_eq!(new_dict.get("id1").as_string(), Some("value1"));
    assert_eq!(new_dict.get("id2").as_string(), Some("value2"));
}

#[test]
fn dict_exact_size_iterator() {
    let mut mut_dict = MutableDict::new();
    mut_dict.at("1").put_string("value1");
    mut_dict.at("2").put_string("value2");
    let dict = mut_dict.as_dict();
    let mut dict_iter = dict.iter();
    dict_iter.next();
    assert_eq!(DictIterator::count(&dict_iter), 1);
    assert_eq!(dict_iter.len(), 2);
}

#[test]
fn dict_from_iterator() {
    let dict: MutableDict = Fleece::parse_json(r#"{"1": "value1","f":12.34}"#)
        .unwrap()
        .as_dict()
        .iter()
        .collect();

    assert_eq!(dict.count(), 2);
    assert_eq!(dict.get("1").as_string(), Some("value1"));

    let mut mut_dict = MutableDict::new();
    mut_dict.at("1").put_string("value1");
    mut_dict.at("2").put_string("value2");

    let dict: MutableDict = mut_dict.as_dict().iter().collect();
    assert_eq!(dict.count(), 2);
    assert_eq!(dict.get("1").as_string(), Some("value1"));
}

#[test]
fn array_at() {
    let mut mut_arr = MutableArray::new();
    assert!(mut_arr.at(0).is_none());
    mut_arr.append().put_string("value1");
    assert!(mut_arr.at(0).is_some());
}

#[test]
fn array_exact_size_iterator() {
    let mut mut_arr = MutableArray::new();
    mut_arr.append().put_string("value1");
    mut_arr.append().put_string("value2");
    let arr = mut_arr.as_array();
    let mut arr_iter = arr.iter();
    arr_iter.next();
    assert_eq!(ArrayIterator::count(&arr_iter), 1);
    assert_eq!(arr_iter.len(), 2);
}

#[test]
fn array_from_iterator() {
    let arr: MutableArray = Fleece::parse_json(r#"["value1","value2"]"#)
        .unwrap()
        .as_array()
        .iter()
        .collect();

    assert_eq!(arr.count(), 2);
    assert_eq!(arr.get(0).as_string(), Some("value1"));

    let mut mut_arr = MutableArray::new();
    mut_arr.append().put_string("value1");
    mut_arr.append().put_string("value2");

    let arr: MutableArray = mut_arr.as_array().iter().collect();
    assert_eq!(arr.count(), 2);
    assert_eq!(arr.get(0).as_string(), Some("value1"));
}
