// Fleece mutable-object API bindings, for Couchbase Lite document properties
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
    CblRef, CouchbaseLiteError, Error, ErrorCode, Result, encryptable,
    slice::{from_bytes, from_str},
    c_api::{
        FLArray_AsMutable, FLArray_MutableCopy, FLDict_AsMutable, FLDict_MutableCopy,
        FLMutableArray, FLMutableArray_Append, FLMutableArray_Insert, FLMutableArray_IsChanged,
        FLMutableArray_New, FLMutableArray_Remove, FLMutableArray_Set, FLMutableDict,
        FLMutableDict_IsChanged, FLMutableDict_New, FLMutableDict_Remove, FLMutableDict_RemoveAll,
        FLMutableDict_Set, FLSlot, FLSlot_SetBool, FLSlot_SetDouble, FLSlot_SetEncryptableValue,
        FLSlot_SetInt, FLSlot_SetNull, FLSlot_SetString, FLSlot_SetValue, FLValue, FLValue_Release,
        FLValue_Retain,
    },
    fleece::{Array, ArrayIterator, Dict, DictIterator, DictKey, FleeceReference, Value},
    encryptable::Encryptable,
};

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::ptr;

#[derive(Debug, Clone, Copy)]
pub enum CopyFlags {
    Default = 0,            // Shallow copy of mutable values
    Deep = 1,               // Deep copy of mutable values
    CopyImmutables = 2,     // Make copies of immutable values too
    DeepCopyImmutables = 3, // The works
}

//////// MUTABLE ARRAY:

pub struct MutableArray {
    pub(crate) cbl_ref: FLMutableArray,
}

impl CblRef for MutableArray {
    type Output = FLMutableArray;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl MutableArray {
    pub fn new() -> Self {
        unsafe {
            Self {
                cbl_ref: FLMutableArray_New(),
            }
        }
    }

    pub fn from_array(array: &Array) -> Self {
        Self::from_array_(array, CopyFlags::Default)
    }

    pub fn from_array_(array: &Array, flags: CopyFlags) -> Self {
        unsafe {
            Self {
                cbl_ref: FLArray_MutableCopy(array.get_ref(), flags as u32),
            }
        }
    }

    pub(crate) unsafe fn adopt(array: FLMutableArray) -> Self {
        FLValue_Retain(array as FLValue);
        Self { cbl_ref: array }
    }

    pub fn is_changed(&self) -> bool {
        unsafe { FLMutableArray_IsChanged(self.get_ref()) }
    }

    pub fn at(&mut self, index: u32) -> Option<Slot> {
        if self.count() > index {
            Some(unsafe {
                Slot {
                    cbl_ref: FLMutableArray_Set(self.get_ref(), index),
                    owner: PhantomData,
                }
            })
        } else {
            None
        }
    }

    pub fn append(&mut self) -> Slot {
        unsafe {
            Slot {
                cbl_ref: FLMutableArray_Append(self.get_ref()),
                owner: PhantomData,
            }
        }
    }

    pub fn insert(&mut self, index: u32) -> Result<()> {
        if self.count() > index {
            unsafe { FLMutableArray_Insert(self.get_ref(), index, 1) }
            Ok(())
        } else {
            Err(Error {
                code: ErrorCode::CouchbaseLite(CouchbaseLiteError::MemoryError),
                internal_info: None,
            })
        }
    }

    pub fn remove(&mut self, index: u32) {
        unsafe { FLMutableArray_Remove(self.get_ref(), index, 1) }
    }

    pub fn remove_all(&mut self) {
        unsafe { FLMutableArray_Remove(self.get_ref(), 0, self.count()) }
    }
}

// "Inherited" API:
impl MutableArray {
    pub fn as_array(&self) -> Array {
        Array::wrap(self.get_ref())
    }
    pub fn count(&self) -> u32 {
        self.as_array().count()
    }
    pub fn empty(&self) -> bool {
        self.as_array().empty()
    }
    pub fn get(&self, index: u32) -> Value {
        self.as_array().get(index)
    }
    pub fn iter(&self) -> ArrayIterator {
        self.as_array().iter()
    }
}

impl FleeceReference for MutableArray {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref() as FLValue
    }
}

impl Clone for MutableArray {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                cbl_ref: FLValue_Retain(self.get_ref() as FLValue) as FLMutableArray,
            }
        }
    }
}

impl Drop for MutableArray {
    fn drop(&mut self) {
        unsafe {
            FLValue_Release(self.get_ref() as FLValue);
        }
    }
}

impl Default for MutableArray {
    fn default() -> Self {
        Self {
            cbl_ref: ptr::null_mut(),
        }
    }
}

impl PartialEq for MutableArray {
    fn eq(&self, other: &Self) -> bool {
        self.as_value() == other.as_value()
    }
}

impl Eq for MutableArray {}

impl std::ops::Not for MutableArray {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl fmt::Debug for MutableArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutableArray")
            .field("count", &self.count())
            .finish()
    }
}

impl fmt::Display for MutableArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_value().to_json())
    }
}

impl IntoIterator for MutableArray {
    type Item = Value;
    type IntoIter = ArrayIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Mutable API additions for Array:
impl Array {
    pub fn as_mutable(self) -> Option<MutableArray> {
        unsafe {
            let md = FLArray_AsMutable(self.get_ref());
            if md.is_null() {
                None
            } else {
                Some(MutableArray::adopt(md))
            }
        }
    }

    pub fn mutable_copy(&self) -> MutableArray {
        MutableArray::from_array(self)
    }
}

//////// MUTABLE DICT:

pub struct MutableDict {
    pub(crate) cbl_ref: FLMutableDict,
}

impl CblRef for MutableDict {
    type Output = FLMutableDict;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl MutableDict {
    pub fn new() -> Self {
        unsafe {
            Self {
                cbl_ref: FLMutableDict_New(),
            }
        }
    }

    pub fn from_dict(dict: &Dict) -> Self {
        Self::from_dict_(dict, CopyFlags::Default)
    }

    pub fn from_dict_(dict: &Dict, flags: CopyFlags) -> Self {
        unsafe {
            Self {
                cbl_ref: FLDict_MutableCopy(dict.get_ref(), flags as u32),
            }
        }
    }

    pub(crate) unsafe fn adopt(dict: FLMutableDict) -> Self {
        FLValue_Retain(dict as FLValue);
        Self { cbl_ref: dict }
    }

    pub fn is_changed(&self) -> bool {
        unsafe { FLMutableDict_IsChanged(self.get_ref()) }
    }

    pub fn at<'s>(&'s mut self, key: &str) -> Slot<'s> {
        unsafe {
            Slot {
                cbl_ref: FLMutableDict_Set(self.get_ref(), from_str(key).get_ref()),
                owner: PhantomData,
            }
        }
    }

    pub fn remove(&mut self, key: &str) {
        unsafe { FLMutableDict_Remove(self.get_ref(), from_str(key).get_ref()) }
    }

    pub fn remove_all(&mut self) {
        unsafe { FLMutableDict_RemoveAll(self.get_ref()) }
    }

    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.iter()
            .map(|tuple| {
                (
                    tuple.0.to_string(),
                    String::from(tuple.1.as_string().unwrap_or("")),
                )
            })
            .collect::<HashMap<String, String>>()
    }

    pub fn set_encryptable_value(dict: &Self, key: &str, encryptable: &Encryptable) {
        unsafe {
            FLSlot_SetEncryptableValue(
                FLMutableDict_Set(dict.get_ref(), from_str(key).get_ref()),
                encryptable.get_ref(),
            );
        }
    }

    pub fn from_hashmap(map: &HashMap<String, String>) -> Self {
        let mut dict = Self::new();
        map.iter()
            .for_each(|(key, value)| dict.at(key.as_str()).put_string(value.as_str()));
        dict
    }
}

// "Inherited" API:
impl MutableDict {
    pub fn as_dict(&self) -> Dict {
        Dict::wrap(self.get_ref(), self)
    }
    pub fn count(&self) -> u32 {
        self.as_dict().count()
    }
    pub fn empty(&self) -> bool {
        self.as_dict().empty()
    }
    pub fn get(&self, key: &str) -> Value {
        self.as_dict().get(key)
    }
    pub fn get_key(&self, key: &mut DictKey) -> Value {
        self.as_dict().get_key(key)
    }
    pub fn iter(&self) -> DictIterator {
        self.as_dict().iter()
    }
}

impl FleeceReference for MutableDict {
    fn _fleece_ref(&self) -> FLValue {
        self.get_ref() as FLValue
    }
}

impl Clone for MutableDict {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                cbl_ref: FLValue_Retain(self.get_ref() as FLValue) as FLMutableDict,
            }
        }
    }
}

impl Drop for MutableDict {
    fn drop(&mut self) {
        unsafe {
            FLValue_Release(self.get_ref() as FLValue);
        }
    }
}

impl Default for MutableDict {
    fn default() -> Self {
        Self {
            cbl_ref: ptr::null_mut(),
        }
    }
}

impl PartialEq for MutableDict {
    fn eq(&self, other: &Self) -> bool {
        self.as_value() == other.as_value()
    }
}

impl Eq for MutableDict {}

impl std::ops::Not for MutableDict {
    type Output = bool;
    fn not(self) -> bool {
        self.get_ref().is_null()
    }
}

impl fmt::Debug for MutableDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MutableDict")
            .field("count", &self.count())
            .finish()
    }
}

impl fmt::Display for MutableDict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_value().to_json())
    }
}

impl IntoIterator for MutableDict {
    type Item = (String, Value);
    type IntoIter = DictIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// Mutable API for Dict:
impl Dict {
    pub fn as_mutable(self) -> Option<MutableDict> {
        unsafe {
            let md = FLDict_AsMutable(self.get_ref());
            if md.is_null() {
                None
            } else {
                Some(MutableDict::adopt(md))
            }
        }
    }

    pub fn mutable_copy(&self) -> MutableDict {
        MutableDict::from_dict(self)
    }
}

//////// SLOT:

/** A reference to an element of a MutableArray or MutableDict,
for the sole purpose of storing a value in it. */
pub struct Slot<'s> {
    pub(crate) cbl_ref: FLSlot,
    owner: PhantomData<&'s mut MutableDict>,
}

impl<'s> CblRef for Slot<'s> {
    type Output = FLSlot;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl<'s> Slot<'s> {
    pub fn put_null(self) {
        unsafe { FLSlot_SetNull(self.get_ref()) }
    }

    pub fn put_bool(self, value: bool) {
        unsafe { FLSlot_SetBool(self.get_ref(), value) }
    }

    pub fn put_i64<INT: Into<i64>>(self, value: INT) {
        unsafe { FLSlot_SetInt(self.get_ref(), value.into()) }
    }

    pub fn put_f64<F: Into<f64>>(self, value: F) {
        unsafe { FLSlot_SetDouble(self.get_ref(), value.into()) }
    }

    pub fn put_string<STR: AsRef<str>>(self, value: STR) {
        unsafe { FLSlot_SetString(self.get_ref(), from_str(value.as_ref()).get_ref()) }
    }

    pub fn put_data<DATA: AsRef<[u8]>>(self, value: DATA) {
        unsafe { FLSlot_SetString(self.get_ref(), from_bytes(value.as_ref()).get_ref()) }
    }

    pub fn put_value<VALUE: FleeceReference>(self, value: &VALUE) {
        unsafe { FLSlot_SetValue(self.get_ref(), value._fleece_ref()) }
    }

    pub fn put_encrypt(self, value: &encryptable::Encryptable) {
        unsafe { FLSlot_SetEncryptableValue(self.get_ref(), value.get_ref()) }
    }
}
