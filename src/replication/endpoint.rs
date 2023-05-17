use crate::{
    CblRef, Database, check_error,
    c_api::{CBLEndpoint, CBLError, CBLEndpoint_CreateWithLocalDB, CBLEndpoint_CreateWithURL},
    error::Result,
    slice::from_str,
};

/** Represents the location of a database to replicate with. */
#[derive(Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub(crate) cbl_ref: *mut CBLEndpoint,
    pub url: Option<String>,
}

impl CblRef for Endpoint {
    type Output = *mut CBLEndpoint;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Endpoint {
    pub fn new_with_url(url: &str) -> Result<Self> {
        unsafe {
            let mut error = CBLError::default();
            let endpoint: *mut CBLEndpoint =
                CBLEndpoint_CreateWithURL(from_str(url).get_ref(), std::ptr::addr_of_mut!(error));

            check_error(&error).map(|_| Self {
                cbl_ref: endpoint,
                url: Some(url.to_string()),
            })
        }
    }

    pub fn new_with_local_db(db: &Database) -> Self {
        unsafe {
            Self {
                cbl_ref: CBLEndpoint_CreateWithLocalDB(db.get_ref()),
                url: None,
            }
        }
    }
}

impl Clone for Endpoint {
    fn clone(&self) -> Self {
        Self {
            cbl_ref: self.cbl_ref,
            url: self.url.clone(),
        }
    }
}
