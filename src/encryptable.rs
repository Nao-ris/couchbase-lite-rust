use crate::{
    c_api::{
        CBLEncryptable, CBLEncryptable_CreateWithArray, CBLEncryptable_CreateWithBool,
        CBLEncryptable_CreateWithDict, CBLEncryptable_CreateWithDouble,
        CBLEncryptable_CreateWithFloat, CBLEncryptable_CreateWithInt,
        CBLEncryptable_CreateWithNull, CBLEncryptable_CreateWithString,
        CBLEncryptable_CreateWithUInt, CBLEncryptable_CreateWithValue, CBLEncryptable_Properties,
        CBLEncryptable_Value, FLSlice_Copy,
    },
    slice::from_str,
    Array, CblRef, Dict, Value, release, retain,
};

pub struct Encryptable {
    cbl_ref: *mut CBLEncryptable,
}

impl CblRef for Encryptable {
    type Output = *mut CBLEncryptable;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl From<*mut CBLEncryptable> for Encryptable {
    fn from(cbl_ref: *mut CBLEncryptable) -> Self {
        Self::retain(cbl_ref)
    }
}

impl Encryptable {
    pub(crate) fn retain(cbl_ref: *mut CBLEncryptable) -> Self {
        Self {
            cbl_ref: unsafe { retain(cbl_ref) },
        }
    }

    pub fn create_with_null() -> Self {
        unsafe { CBLEncryptable_CreateWithNull().into() }
    }

    pub fn create_with_bool(value: bool) -> Self {
        unsafe { CBLEncryptable_CreateWithBool(value).into() }
    }

    pub fn create_with_int(value: i64) -> Self {
        unsafe { CBLEncryptable_CreateWithInt(value).into() }
    }

    pub fn create_with_uint(value: u64) -> Self {
        unsafe { CBLEncryptable_CreateWithUInt(value).into() }
    }

    pub fn create_with_float(value: f32) -> Self {
        unsafe { CBLEncryptable_CreateWithFloat(value).into() }
    }

    pub fn create_with_double(value: f64) -> Self {
        unsafe { CBLEncryptable_CreateWithDouble(value).into() }
    }

    pub fn create_with_string(value: &str) -> Self {
        unsafe {
            let slice = from_str(value);
            let copy_slice = FLSlice_Copy(slice.get_ref());
            let final_slice = copy_slice.as_slice();
            CBLEncryptable_CreateWithString(final_slice).into()
        }
    }

    pub fn create_with_value(value: Value) -> Self {
        unsafe { CBLEncryptable_CreateWithValue(value.get_ref()).into() }
    }

    pub fn create_with_array(value: Array) -> Self {
        unsafe { CBLEncryptable_CreateWithArray(value.get_ref()).into() }
    }

    pub fn create_with_dict(value: Dict) -> Self {
        unsafe { CBLEncryptable_CreateWithDict(value.get_ref()).into() }
    }

    pub fn get_value(&self) -> Value {
        unsafe { Value::wrap(CBLEncryptable_Value(self.get_ref()), self) }
    }

    pub fn get_properties(&self) -> Dict {
        unsafe { Dict::wrap(CBLEncryptable_Properties(self.get_ref()), self) }
    }
}

impl Drop for Encryptable {
    fn drop(&mut self) {
        unsafe {
            release(self.get_ref().cast::<CBLEncryptable>());
        }
    }
}

impl Clone for Encryptable {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                cbl_ref: retain(self.get_ref().cast::<CBLEncryptable>()),
            }
        }
    }
}
