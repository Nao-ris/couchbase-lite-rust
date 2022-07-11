use super::c_api::*;
use super::slice::*;
use super::*;

pub struct Encryptable {
    pub _ref: *mut CBLEncryptable,
}

impl From<*mut CBLEncryptable> for Encryptable {
    fn from(_ref: *mut CBLEncryptable) -> Self {
        Encryptable { _ref: unsafe { retain(_ref) } }
    }
}

impl Encryptable {
    pub fn new(_ref: *mut CBLEncryptable) -> Self {
        Encryptable {
            _ref: unsafe { retain(_ref) }
        }
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
            let stri = value.as_str();
            println!("Antoine building: {:?}", stri);
            let slice = as_slice(stri);
            println!("Antoine building: {:?}", slice.to_string());
            let copy_slice = FLSlice_Copy(slice);
            //println!("Antoine building: {:?}", copy_slice.to_string());
            let final_slice = copy_slice.as_slice();
            println!("Antoine building: {:?}", final_slice.to_string());
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


    // Helper for testing purposes
    pub fn as_string(&self) -> Option<&str> {
        self.get_value().as_string()
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
