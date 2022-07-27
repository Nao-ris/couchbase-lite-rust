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
        FLValue_GetType, FLValue_IsEqual, FLValue_IsInteger, FLValue_ToJSON, _FLValue,
    },
    encryptable::Encryptable,
};

use enum_primitive::FromPrimitive;
use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr;
use std::str;

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

    pub fn as_array(&self) -> Array {
        self.root().as_array()
    }

    pub fn as_dict(&self) -> Dict {
        self.root().as_dict()
    }

    pub fn data<'a>(&self) -> &'a [u8] {
        unsafe { FLDoc_GetData(self.get_ref()).as_byte_array().unwrap() }
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
    #[derive(Debug, PartialEq, Clone, Copy)]
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
            owner: PhantomData,
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
pub struct Value<'a> {
    pub(crate) cbl_ref: FLValue,
    pub(crate) owner: PhantomData<&'a Fleece>,
}

impl<'a> CblRef for Value<'a> {
    type Output = FLValue;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl<'a> Value<'a> {
    pub const UNDEFINED: Value<'static> = Value {
        cbl_ref: ptr::null(),
        owner: PhantomData,
    };

    pub(crate) const fn wrap<T>(value: FLValue, _owner: &T) -> Self {
        Self {
            cbl_ref: value,
            owner: PhantomData,
        }
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

    pub fn as_string(&self) -> Option<&'a str> {
        unsafe { FLValue_AsString(self.get_ref()).as_str() }
    }

    pub fn as_data(&self) -> Option<&'a [u8]> {
        unsafe { FLValue_AsData(self.get_ref()).as_byte_array() }
    }

    pub fn as_array(&self) -> Array<'a> {
        unsafe {
            Array {
                cbl_ref: FLValue_AsArray(self.get_ref()),
                owner: self.owner,
            }
        }
    }

    pub fn as_dict(&self) -> Dict<'a> {
        unsafe {
            Dict {
                cbl_ref: FLValue_AsDict(self.get_ref()),
                owner: self.owner,
            }
        }
    }

    pub fn get_encryptable_value(&self) -> Encryptable {
        unsafe {
            let encryptable = FLDict_GetEncryptableValue(FLValue_AsDict(self.get_ref()));
            Encryptable::retain(encryptable as *mut CBLEncryptable)
        }
    }
}

impl<'a> FleeceReference for Value<'a> {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref()
    }
}

impl<'a> Default for Value<'a> {
    fn default() -> Self {
        Self::UNDEFINED
    }
}

impl<'a> PartialEq for Value<'a> {
    fn eq(&self, other: &Self) -> bool {
        unsafe { FLValue_IsEqual(self.get_ref(), other.get_ref()) }
    }
}

impl<'a> Eq for Value<'a> {}

impl<'a> std::ops::Not for Value<'a> {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Value")
            .field("type", &self.get_type())
            .finish()
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_json())
    }
}

//////// ARRAY

/** A Fleece array value. */
#[derive(Clone, Copy)]
pub struct Array<'a> {
    pub(crate) cbl_ref: FLArray,
    pub(crate) owner: PhantomData<&'a Fleece>,
}

impl<'a> CblRef for Array<'a> {
    type Output = FLArray;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl<'a> Array<'a> {
    pub(crate) const fn wrap<T>(array: FLArray, _owner: &T) -> Self {
        Self {
            cbl_ref: array,
            owner: PhantomData,
        }
    }

    pub fn count(&self) -> u32 {
        unsafe { FLArray_Count(self.get_ref()) }
    }
    pub fn empty(&self) -> bool {
        unsafe { FLArray_IsEmpty(self.get_ref()) }
    }

    pub fn get(&self, index: u32) -> Value<'a> {
        unsafe {
            Value {
                cbl_ref: FLArray_Get(self.get_ref(), index),
                owner: self.owner,
            }
        }
    }

    pub fn iter(&self) -> ArrayIterator<'a> {
        unsafe {
            let mut i = MaybeUninit::<FLArrayIterator>::uninit();
            FLArrayIterator_Begin(self.get_ref(), i.as_mut_ptr());
            ArrayIterator {
                innards: i.assume_init(),
                owner: self.owner,
                len: self.count() as usize,
            }
        }
    }
}

impl<'a> FleeceReference for Array<'a> {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref().cast::<_FLValue>()
    }
}

impl<'a> Default for Array<'a> {
    fn default() -> Self {
        Self {
            cbl_ref: ptr::null(),
            owner: PhantomData,
        }
    }
}

impl<'a> PartialEq for Array<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_value() == other.as_value()
    }
}

impl<'a> Eq for Array<'a> {}

impl<'a> std::ops::Not for Array<'a> {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl<'a> fmt::Debug for Array<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Array")
            .field("count", &self.count())
            .finish()
    }
}

impl<'a> fmt::Display for Array<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_value().to_json())
    }
}

impl<'a> IntoIterator for Array<'a> {
    type Item = Value<'a>;
    type IntoIter = ArrayIterator<'a>;
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

pub struct ArrayIterator<'a> {
    innards: FLArrayIterator,
    owner: PhantomData<&'a Fleece>,
    len: usize,
}

impl<'a> ArrayIterator<'a> {
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

impl<'a> Iterator for ArrayIterator<'a> {
    type Item = Value<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let val = FLArrayIterator_GetValue(&self.innards);
            if val.is_null() {
                return None;
            }
            FLArrayIterator_Next(&mut self.innards);
            Some(Value {
                cbl_ref: val,
                owner: PhantomData,
            })
        }
    }
}

impl<'a> std::iter::FusedIterator for ArrayIterator<'a> {}

impl<'a> ExactSizeIterator for ArrayIterator<'a> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a> std::iter::FromIterator<Value<'a>> for MutableArray {
    fn from_iter<I: IntoIterator<Item = Value<'a>>>(iter: I) -> Self {
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
pub struct Dict<'a> {
    pub(crate) cbl_ref: FLDict,
    pub(crate) owner: PhantomData<&'a Fleece>,
}

impl<'a> CblRef for Dict<'a> {
    type Output = FLDict;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl<'a> Dict<'a> {
    pub(crate) const fn new(dict: FLDict) -> Self {
        Self {
            cbl_ref: dict,
            owner: PhantomData,
        }
    }

    pub(crate) const fn wrap<T>(dict: FLDict, _owner: &T) -> Self {
        Self {
            cbl_ref: dict,
            owner: PhantomData,
        }
    }

    pub fn as_value(&self) -> Value<'a> {
        Value {
            cbl_ref: self.get_ref().cast::<_FLValue>(),
            owner: self.owner,
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

    pub fn get(&self, key: &str) -> Value<'a> {
        unsafe {
            Value {
                cbl_ref: FLDict_Get(self.get_ref(), from_str(key).get_ref()),
                owner: self.owner,
            }
        }
    }

    pub fn get_key(&self, key: &mut DictKey) -> Value<'a> {
        unsafe {
            Value {
                cbl_ref: FLDict_GetWithKey(self.get_ref(), &mut key.innards),
                owner: self.owner,
            }
        }
    }

    pub fn get_encryptable_value(&self) -> Encryptable {
        unsafe {
            let encryptable = FLDict_GetEncryptableValue(self.get_ref());
            Encryptable::retain(encryptable as *mut CBLEncryptable)
        }
    }

    pub fn iter(&self) -> DictIterator<'a> {
        unsafe {
            let mut i = MaybeUninit::<FLDictIterator>::uninit();
            FLDictIterator_Begin(self.get_ref(), i.as_mut_ptr());
            DictIterator {
                innards: i.assume_init(),
                owner: self.owner,
                len: self.count() as usize,
            }
        }
    }

    pub fn to_keys_hash_set(&self) -> HashSet<String> {
        self.into_iter()
            .map(|tuple| tuple.0.to_string())
            .collect::<HashSet<String>>()
    }
}

impl<'a> FleeceReference for Dict<'a> {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref().cast::<_FLValue>()
    }
}

impl<'a> Default for Dict<'a> {
    fn default() -> Self {
        Self {
            cbl_ref: ptr::null(),
            owner: PhantomData,
        }
    }
}

impl<'a> PartialEq for Dict<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_value() == other.as_value()
    }
}

impl<'a> Eq for Dict<'a> {}

impl<'a> std::ops::Not for Dict<'a> {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl<'a> fmt::Debug for Dict<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dict")
            .field("count", &self.count())
            .finish()
    }
}

impl<'a> fmt::Display for Dict<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_value().to_json())
    }
}

impl<'a> IntoIterator for Dict<'a> {
    type Item = (&'a str, Value<'a>);
    type IntoIter = DictIterator<'a>;
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

pub struct DictIterator<'a> {
    innards: FLDictIterator,
    owner: PhantomData<&'a Fleece>,
    len: usize,
}

impl<'a> DictIterator<'a> {
    pub fn count(&self) -> u32 {
        unsafe { FLDictIterator_GetCount(&self.innards) }
    }
}

impl<'a> Iterator for DictIterator<'a> {
    type Item = (&'a str, Value<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let val = FLDictIterator_GetValue(&self.innards);
            if val.is_null() {
                return None;
            }
            let key = FLDictIterator_GetKeyString(&self.innards).as_str().unwrap();
            FLDictIterator_Next(&mut self.innards);
            Some((
                key,
                Value {
                    cbl_ref: val,
                    owner: PhantomData,
                },
            ))
        }
    }
}

impl<'a> std::iter::FusedIterator for DictIterator<'a> {}

impl<'a> ExactSizeIterator for DictIterator<'a> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<'a> std::iter::FromIterator<(&'a str, Value<'a>)> for MutableDict {
    fn from_iter<T: IntoIterator<Item = (&'a str, Value<'a>)>>(iter: T) -> Self {
        let mut mut_dict = Self::new();
        for (key, value) in iter {
            mut_dict.at(key).put_value(&value);
        }
        mut_dict
    }
}
