// Fleece API bindings, for Couchbase Lite document properties
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

use crate::{
    Blob, CblRef, MutableArray, MutableDict, Timestamp,
    slice::{NULL_SLICE, from_bytes, from_str},
    error::{Error, Result},
    c_api::{
        CBLEncryptable, FLArray, FLArrayIterator, FLArrayIterator_Begin, FLArrayIterator_GetCount,
        FLArrayIterator_GetValue, FLArrayIterator_GetValueAt, FLArrayIterator_Next, FLArray_Count,
        FLArray_Get, FLArray_IsEmpty, FLDict, FLDictIterator, FLDictIterator_Begin,
        FLDictIterator_GetCount, FLDictIterator_GetKeyString, FLDictIterator_GetValue,
        FLDictIterator_Next, FLDictKey, FLDictKey_GetString, FLDictKey_Init, FLDict_Count,
        FLDict_Get, FLDict_GetEncryptableValue, FLDict_GetWithKey, FLDict_IsBlob, FLDict_IsEmpty,
        FLDict_IsEncryptableValue, FLDoc, FLDoc_FromJSON, FLDoc_FromResultData, FLDoc_GetData,
        FLDoc_GetRoot, FLDoc_Release, FLDoc_Retain, FLError, FLError_kFLInvalidData, FLSlice_Copy,
        FLValue, FLValue_AsArray, FLValue_AsBool, FLValue_AsData, FLValue_AsDict, FLValue_AsDouble,
        FLValue_AsFloat, FLValue_AsInt, FLValue_AsString, FLValue_AsTimestamp, FLValue_AsUnsigned,
        FLValue_GetType, FLValue_IsEqual, FLValue_IsInteger, FLValue_IsUnsigned, FLValue_IsDouble,
        FLValue_IsMutable, FLValue_ToJSON, _FLValue, FLValue_FindDoc, FLDictIterator_End,
    },
    encryptable::Encryptable,
};

use enum_primitive::FromPrimitive;
use std::collections::HashSet;
use std::fmt;
use std::mem::MaybeUninit;
use std::ptr;
use std::str;
use retain;

//////// CONTAINER

pub enum Trust {
    Untrusted,
    Trusted,
}

/// Equivalent to FLDoc
pub struct Fleece {
    pub(crate) cbl_ref: FLDoc,
}

impl CblRef for Fleece {
    type Output = FLDoc;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Fleece {
    pub fn parse(data: &[u8], trust: Trust) -> Result<Self> {
        unsafe {
            let copied = FLSlice_Copy(from_bytes(data).get_ref());
            let doc = FLDoc_FromResultData(copied, trust as u32, ptr::null_mut(), NULL_SLICE);
            if doc.is_null() {
                return Err(Error::fleece_error(FLError_kFLInvalidData));
            }
            Ok(Self { cbl_ref: doc })
        }
    }

    pub fn parse_json(json: &str) -> Result<Self> {
        unsafe {
            let mut error: FLError = 0;
            let doc = FLDoc_FromJSON(from_str(json).get_ref(), &mut error);
            if doc.is_null() {
                return Err(Error::fleece_error(error));
            }
            Ok(Self { cbl_ref: doc })
        }
    }

    pub fn root(&self) -> Value {
        unsafe { Value::wrap(FLDoc_GetRoot(self.get_ref()), self) }
    }

    pub fn as_shared_key() {
        todo!("To be implemented when needed")
    }

    pub fn as_array(&self) -> Array {
        self.root().as_array()
    }

    pub fn as_dict(&self) -> Dict {
        self.root().as_dict()
    }

    pub fn data(&self) -> &[u8] {
        unsafe { FLDoc_GetData(self.get_ref()).as_byte_array().unwrap() }
    }

    pub fn wrap(doc: FLDoc) -> Self {
        Self {
            cbl_ref: unsafe { retain(doc) },
        }
    }
}

impl Drop for Fleece {
    fn drop(&mut self) {
        unsafe {
            FLDoc_Release(self.get_ref());
        }
    }
}

impl Clone for Fleece {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                cbl_ref: FLDoc_Retain(self.get_ref()),
            }
        }
    }
}

//////// VALUE

enum_from_primitive! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum ValueType {
        Undefined = -1,  // Type of a NULL pointer, i.e. no such value, like JSON `undefined`
        Null = 0,        // Equivalent to a JSON 'null'
        Bool,            // A `true` or `false` value
        Number,          // A numeric value, either integer or floating-point
        String,          // A string
        Data,            // Binary data (no JSON equivalent)
        Array,           // An array of values
        Dict             // A mapping of strings to values
    }
}

/** A trait for Value, Array and Dict. */
pub trait FleeceReference:
    Default + PartialEq + Eq + std::ops::Not + fmt::Debug + fmt::Display
{
    fn _fleece_ref(&self) -> FLValue; // not for public consumption

    fn as_value(&self) -> Value {
        Value {
            cbl_ref: self._fleece_ref(),
        }
    }

    fn to_json(&self) -> String {
        unsafe { FLValue_ToJSON(self._fleece_ref()).to_string().unwrap() }
    }

    // Blob accessors:

    fn is_blob(&self) -> bool {
        unsafe { FLDict_IsBlob(FLValue_AsDict(self._fleece_ref())) }
    }

    fn as_blob(&self) -> Option<Blob> {
        Blob::from_value(self)
    }
}

/** A Fleece value. It could be any type, including Undefined (empty). */
#[derive(Clone, Copy)]
pub struct Value {
    pub(crate) cbl_ref: FLValue,
}

impl CblRef for Value {
    type Output = FLValue;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Value {
    pub const UNDEFINED: Self = Self {
        cbl_ref: ptr::null(),
    };

    pub(crate) const fn wrap<T>(value: FLValue, _owner: &T) -> Self {
        Self { cbl_ref: value }
    }

    pub fn get_type(&self) -> ValueType {
        unsafe { ValueType::from_i32(FLValue_GetType(self.get_ref())).unwrap() }
    }

    pub fn is_type(&self, t: ValueType) -> bool {
        self.get_type() == t
    }

    pub fn is_number(&self) -> bool {
        self.is_type(ValueType::Number)
    }

    pub fn is_integer(&self) -> bool {
        unsafe { FLValue_IsInteger(self.get_ref()) }
    }

    pub fn is_unsigned(&self) -> bool {
        unsafe { FLValue_IsUnsigned(self.get_ref()) }
    }

    pub fn is_double(&self) -> bool {
        unsafe { FLValue_IsDouble(self.get_ref()) }
    }

    pub fn is_mutable(&self) -> bool {
        unsafe { FLValue_IsMutable(self.get_ref()) }
    }

    pub fn is_encryptable(&self) -> bool {
        unsafe { FLDict_IsEncryptableValue(FLValue_AsDict(self.get_ref())) }
    }

    pub fn as_i64(&self) -> Option<i64> {
        if self.is_integer() {
            Some(self.as_i64_or_0())
        } else {
            None
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        if self.is_integer() {
            Some(self.as_u64_or_0())
        } else {
            None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        if self.is_number() {
            Some(self.as_f64_or_0())
        } else {
            None
        }
    }
    pub fn as_f32(&self) -> Option<f32> {
        if self.is_number() {
            Some(self.as_f32_or_0())
        } else {
            None
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if self.is_type(ValueType::Bool) {
            Some(self.as_bool_or_false())
        } else {
            None
        }
    }

    pub fn as_i64_or_0(&self) -> i64 {
        unsafe { FLValue_AsInt(self.get_ref()) }
    }
    pub fn as_u64_or_0(&self) -> u64 {
        unsafe { FLValue_AsUnsigned(self.get_ref()) }
    }
    pub fn as_f64_or_0(&self) -> f64 {
        unsafe { FLValue_AsDouble(self.get_ref()) }
    }
    pub fn as_f32_or_0(&self) -> f32 {
        unsafe { FLValue_AsFloat(self.get_ref()) }
    }
    pub fn as_bool_or_false(&self) -> bool {
        unsafe { FLValue_AsBool(self.get_ref()) }
    }

    pub fn as_timestamp(&self) -> Option<Timestamp> {
        unsafe {
            let t = FLValue_AsTimestamp(self.get_ref());
            if t == 0 {
                return None;
            }
            Some(Timestamp(t))
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        unsafe { FLValue_AsString(self.get_ref()).as_str() }
    }

    pub fn as_data(&self) -> Option<&[u8]> {
        unsafe { FLValue_AsData(self.get_ref()).as_byte_array() }
    }

    pub fn as_array(&self) -> Array {
        unsafe {
            Array {
                cbl_ref: FLValue_AsArray(self.get_ref()),
            }
        }
    }

    pub fn as_dict(&self) -> Dict {
        unsafe {
            Dict {
                cbl_ref: FLValue_AsDict(self.get_ref()),
            }
        }
    }

    pub fn get_encryptable_value(&self) -> Encryptable {
        unsafe {
            let encryptable = FLDict_GetEncryptableValue(FLValue_AsDict(self.get_ref()));
            Encryptable::retain(encryptable as *mut CBLEncryptable)
        }
    }

    pub fn find_doc(&self) -> Option<Fleece> {
        let doc = unsafe { FLValue_FindDoc(self.get_ref()) };
        if doc.is_null() {
            return None;
        }
        Some(Fleece::wrap(doc))
    }
}

impl FleeceReference for Value {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref()
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::UNDEFINED
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        unsafe { FLValue_IsEqual(self.get_ref(), other.get_ref()) }
    }
}

impl Eq for Value {}

impl std::ops::Not for Value {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Value")
            .field("type", &self.get_type())
            .finish()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_json())
    }
}

//////// ARRAY

/** A Fleece array value. */
#[derive(Clone, Copy)]
pub struct Array {
    pub(crate) cbl_ref: FLArray,
}

impl CblRef for Array {
    type Output = FLArray;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Array {
    pub(crate) const fn wrap(array: FLArray) -> Self {
        Self { cbl_ref: array }
    }

    pub fn count(&self) -> u32 {
        unsafe { FLArray_Count(self.get_ref()) }
    }
    pub fn empty(&self) -> bool {
        unsafe { FLArray_IsEmpty(self.get_ref()) }
    }

    pub fn get(&self, index: u32) -> Value {
        unsafe {
            Value {
                cbl_ref: FLArray_Get(self.get_ref(), index),
            }
        }
    }

    pub fn iter(&self) -> ArrayIterator {
        unsafe {
            let mut i = MaybeUninit::<FLArrayIterator>::uninit();
            FLArrayIterator_Begin(self.get_ref(), i.as_mut_ptr());
            ArrayIterator {
                innards: i.assume_init(),
                len: self.count() as usize,
            }
        }
    }
}

impl FleeceReference for Array {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref().cast::<_FLValue>()
    }
}

impl Default for Array {
    fn default() -> Self {
        Self {
            cbl_ref: ptr::null(),
        }
    }
}

impl PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.as_value() == other.as_value()
    }
}

impl Eq for Array {}

impl std::ops::Not for Array {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl fmt::Debug for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Array")
            .field("count", &self.count())
            .finish()
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_value().to_json())
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = ArrayIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// This doesn't work because it requires the return value be a ref!
// impl std::ops::Index<usize> for Array {
//     type Output = Value;
//     fn index(&self, index: usize) -> Value { self.get(index) }
// }

//////// ARRAY ITERATOR

pub struct ArrayIterator {
    innards: FLArrayIterator,
    len: usize,
}

impl ArrayIterator {
    pub fn count(&self) -> u32 {
        unsafe { FLArrayIterator_GetCount(&self.innards) }
    }

    pub fn get(&self, index: usize) -> Value {
        unsafe {
            Value::wrap(
                FLArrayIterator_GetValueAt(&self.innards, index as u32),
                self,
            )
        }
    }
}

impl Iterator for ArrayIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let val = FLArrayIterator_GetValue(&self.innards);
            if val.is_null() {
                return None;
            }
            FLArrayIterator_Next(&mut self.innards);
            Some(Value { cbl_ref: val })
        }
    }
}

impl std::iter::FusedIterator for ArrayIterator {}

impl ExactSizeIterator for ArrayIterator {
    fn len(&self) -> usize {
        self.len
    }
}

impl std::iter::FromIterator<Value> for MutableArray {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        let mut c = Self::new();
        for v in iter {
            c.append().put_value(&v);
        }
        c
    }
}

//////// DICT

/** A Fleece dictionary (object) value. */
#[derive(Clone, Copy)]
pub struct Dict {
    pub(crate) cbl_ref: FLDict,
}

impl CblRef for Dict {
    type Output = FLDict;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Dict {
    pub(crate) const fn new(dict: FLDict) -> Self {
        Self { cbl_ref: dict }
    }

    pub(crate) const fn wrap<T>(dict: FLDict, _owner: &T) -> Self {
        Self { cbl_ref: dict }
    }

    pub fn as_value(&self) -> Value {
        Value {
            cbl_ref: self.get_ref().cast::<_FLValue>(),
        }
    }

    pub fn count(&self) -> u32 {
        unsafe { FLDict_Count(self.get_ref()) }
    }
    pub fn empty(&self) -> bool {
        unsafe { FLDict_IsEmpty(self.get_ref()) }
    }

    pub fn is_encryptable(&self) -> bool {
        unsafe { FLDict_IsEncryptableValue(self.get_ref()) }
    }

    pub fn get(&self, key: &str) -> Value {
        unsafe {
            Value {
                cbl_ref: FLDict_Get(self.get_ref(), from_str(key).get_ref()),
            }
        }
    }

    pub fn get_key(&self, key: &mut DictKey) -> Value {
        unsafe {
            Value {
                cbl_ref: FLDict_GetWithKey(self.get_ref(), &mut key.innards),
            }
        }
    }

    pub fn get_encryptable_value(&self) -> Encryptable {
        unsafe {
            let encryptable = FLDict_GetEncryptableValue(self.get_ref());
            Encryptable::retain(encryptable as *mut CBLEncryptable)
        }
    }

    pub fn iter(&self) -> DictIterator {
        unsafe {
            let mut i = MaybeUninit::<FLDictIterator>::uninit();
            FLDictIterator_Begin(self.get_ref(), i.as_mut_ptr());
            DictIterator {
                innards: i.assume_init(),
                len: self.count() as usize,
            }
        }
    }

    pub fn to_keys_hash_set(&self) -> HashSet<String> {
        self.into_iter()
            .map(|tuple| tuple.0)
            .collect::<HashSet<String>>()
    }
}

impl FleeceReference for Dict {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref().cast::<_FLValue>()
    }
}

impl Default for Dict {
    fn default() -> Self {
        Self {
            cbl_ref: ptr::null(),
        }
    }
}

impl PartialEq for Dict {
    fn eq(&self, other: &Self) -> bool {
        self.as_value() == other.as_value()
    }
}

impl Eq for Dict {}

impl std::ops::Not for Dict {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl fmt::Debug for Dict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dict")
            .field("count", &self.count())
            .finish()
    }
}

impl fmt::Display for Dict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_value().to_json())
    }
}

impl IntoIterator for Dict {
    type Item = (String, Value);
    type IntoIter = DictIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

//////// DICT KEY

pub struct DictKey {
    pub(crate) innards: FLDictKey,
}

impl DictKey {
    pub fn new(key: &str) -> Self {
        unsafe {
            Self {
                innards: FLDictKey_Init(from_str(key).get_ref()),
            }
        }
    }

    pub fn string(&self) -> &str {
        unsafe { FLDictKey_GetString(&self.innards).as_str().unwrap() }
    }
}

//////// DICT ITERATOR

pub struct DictIterator {
    innards: FLDictIterator,
    len: usize,
}

impl DictIterator {
    pub fn count(&self) -> u32 {
        unsafe { FLDictIterator_GetCount(&self.innards) }
    }
}

impl Iterator for DictIterator {
    type Item = (String, Value);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let val = FLDictIterator_GetValue(&self.innards);
            if val.is_null() {
                return None;
            }
            let key = FLDictIterator_GetKeyString(&self.innards)
                .as_str()
                .unwrap_or_default();
            FLDictIterator_Next(&mut self.innards);
            Some((key.to_string(), Value { cbl_ref: val }))
        }
    }
}

impl std::iter::FusedIterator for DictIterator {}

impl ExactSizeIterator for DictIterator {
    fn len(&self) -> usize {
        self.len
    }
}

impl Drop for DictIterator {
    fn drop(&mut self) {
        unsafe { FLDictIterator_End(&mut self.innards) };
    }
}

impl std::iter::FromIterator<(String, Value)> for MutableDict {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        let mut mut_dict = Self::new();
        for (key, value) in iter {
            mut_dict.at(&key).put_value(&value);
        }
        mut_dict
    }
}
