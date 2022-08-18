use crate::{
    CblRef, Database,
    c_api::{
        CBLValueIndexConfiguration, CBLDatabase_GetIndexNames, CBLDatabase_DeleteIndex, CBLError,
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

    pub fn delete_index(&self, name: &str) -> Result<bool> {
        let mut err = CBLError::default();
        let slice = from_str(name);
        let r = unsafe { CBLDatabase_DeleteIndex(self.get_ref(), slice.get_ref(), &mut err) };
        if !err {
            return Ok(r);
        }
        failure(err)
    }

    pub fn get_index_names(&self) -> Array {
        let arr = unsafe { CBLDatabase_GetIndexNames(self.get_ref()) };
        Array::wrap(arr)
    }
}
