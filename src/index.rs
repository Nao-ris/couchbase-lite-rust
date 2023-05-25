use crate::{
    CblRef,
    collection::Collection,
    Database, check_error,
    c_api::{
        CBLValueIndexConfiguration, CBLCollection_CreateValueIndex, CBLCollection_DeleteIndex,
        CBLCollection_GetIndexNames, CBLDatabase_GetIndexNames, CBLDatabase_DeleteIndex, CBLError,
        CBLDatabase_CreateValueIndex,
    },
    error::{Result, failure},
    slice::from_str,
    QueryLanguage, Array,
};

pub struct ValueIndexConfiguration {
    cbl_ref: CBLValueIndexConfiguration,
}

impl CblRef for ValueIndexConfiguration {
    type Output = CBLValueIndexConfiguration;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl ValueIndexConfiguration {
    pub fn new(query_language: QueryLanguage, expressions: &str) -> Self {
        let slice = from_str(expressions);
        Self {
            cbl_ref: CBLValueIndexConfiguration {
                expressionLanguage: query_language as u32,
                expressions: slice.get_ref(),
            },
        }
    }
}

impl Database {
    #[deprecated(note = "please use `create_index` on default collection instead")]
    pub fn create_index(&self, name: &str, config: &ValueIndexConfiguration) -> Result<bool> {
        let mut err = CBLError::default();
        let slice = from_str(name);
        let r = unsafe {
            CBLDatabase_CreateValueIndex(
                self.get_ref(),
                slice.get_ref(),
                config.get_ref(),
                &mut err,
            )
        };
        if !err {
            return Ok(r);
        }
        failure(err)
    }

    #[deprecated(note = "please use `delete_index` on default collection instead")]
    pub fn delete_index(&self, name: &str) -> Result<bool> {
        let mut err = CBLError::default();
        let slice = from_str(name);
        let r = unsafe { CBLDatabase_DeleteIndex(self.get_ref(), slice.get_ref(), &mut err) };
        if !err {
            return Ok(r);
        }
        failure(err)
    }

    #[deprecated(note = "please use `get_index_names` on default collection instead")]
    pub fn get_index_names(&self) -> Array {
        let arr = unsafe { CBLDatabase_GetIndexNames(self.get_ref()) };
        Array::wrap(arr)
    }
}

impl Collection {
    pub fn create_index(&self, name: &str, config: &ValueIndexConfiguration) -> Result<bool> {
        let mut err = CBLError::default();
        let slice = from_str(name);
        let r = unsafe {
            CBLCollection_CreateValueIndex(
                self.get_ref(),
                slice.get_ref(),
                config.get_ref(),
                &mut err,
            )
        };
        if !err {
            return Ok(r);
        }
        failure(err)
    }

    pub fn delete_index(&self, name: &str) -> Result<bool> {
        let mut err = CBLError::default();
        let slice = from_str(name);
        let r = unsafe { CBLCollection_DeleteIndex(self.get_ref(), slice.get_ref(), &mut err) };
        if !err {
            return Ok(r);
        }
        failure(err)
    }

    pub fn get_index_names(&self) -> Result<Array> {
        let mut err = CBLError::default();
        let arr = unsafe { CBLCollection_GetIndexNames(self.get_ref(), &mut err) };
        check_error(&err).map(|()| Array::wrap(arr))
    }
}
