use crate::{
    CblRef,
    c_api::{CBLAuthenticator, CBLAuth_CreatePassword, CBLAuth_CreateSession},
    slice::from_str,
};

/** An opaque object representing authentication credentials for a remote server. */
#[derive(Debug, PartialEq, Eq)]
pub struct Authenticator {
    pub(crate) cbl_ref: *mut CBLAuthenticator,
}

impl CblRef for Authenticator {
    type Output = *mut CBLAuthenticator;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Authenticator {
    pub fn create_password(username: &str, password: &str) -> Self {
        unsafe {
            Self {
                cbl_ref: CBLAuth_CreatePassword(
                    from_str(username).get_ref(),
                    from_str(password).get_ref(),
                ),
            }
        }
    }

    pub fn create_session(session_id: &str, cookie_name: &str) -> Self {
        unsafe {
            Self {
                cbl_ref: CBLAuth_CreateSession(
                    from_str(session_id).get_ref(),
                    from_str(cookie_name).get_ref(),
                ),
            }
        }
    }
}

impl Clone for Authenticator {
    fn clone(&self) -> Self {
        Self {
            cbl_ref: self.cbl_ref,
        }
    }
}
