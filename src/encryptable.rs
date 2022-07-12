use super::c_api::*;
use super::slice::*;
use super::*;

pub struct Encryptable {
    _ref: *mut CBLEncryptable,
}

impl From<*mut CBLEncryptable> for Encryptable {
    fn from(_ref: *mut CBLEncryptable) -> Self {
        Self::retain(_ref)
    }
}

impl Encryptable {
    pub fn retain(_ref: *mut CBLEncryptable) -> Self {
        Encryptable {
            _ref: unsafe { retain(_ref) }
        }
    }

    pub(crate) fn get_ref(&self) -> *mut CBLEncryptable {
        self._ref
    }

    pub fn create_with_null() -> Encryptable {
        unsafe { CBLEncryptable_CreateWithNull().into() }
    }

    pub fn create_with_bool(value: bool) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithBool(value).into() }
    }

    pub fn create_with_int(value: i64) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithInt(value).into() }
    }

    pub fn create_with_uint(value: u64) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithUInt(value).into() }
    }

    pub fn create_with_float(value: f32) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithFloat(value).into() }
    }

    pub fn create_with_double(value: f64) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithDouble(value).into() }
    }

    pub fn create_with_string(value: String) -> Encryptable {
        unsafe {
            let slice = as_slice(value.as_str());
            let copy_slice = FLSlice_Copy(slice);
            let final_slice = copy_slice.as_slice();
            CBLEncryptable_CreateWithString(final_slice).into()
        }
    }

    pub fn create_with_value(value: Value) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithValue(value._ref).into() }
    }

    pub fn create_with_array(value: Array) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithArray(value._ref).into() }
    }

    pub fn create_with_dict(value: Dict) -> Encryptable {
        unsafe { CBLEncryptable_CreateWithDict(value._ref).into() }
    }

    pub fn get_value(&self) -> Value {
        unsafe { Value::wrap(CBLEncryptable_Value(self._ref), self) }
    }

    pub fn get_properties(&self) -> Dict {
        unsafe { Dict::wrap(CBLEncryptable_Properties(self._ref), self) }
    }
}

impl Drop for Encryptable {
    fn drop(&mut self) {
        unsafe {
            release(self._ref as *mut CBLEncryptable);
        }
    }
}

impl Clone for Encryptable {
    fn clone(&self) -> Self {
        unsafe {
            Encryptable {
                _ref: retain(self._ref as *mut CBLEncryptable),
            }
        }
    }
}
