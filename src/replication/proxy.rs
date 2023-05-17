#![allow(non_upper_case_globals)]

use crate::{
    CblRef,
    c_api::{CBLProxySettings, CBLProxyType, kCBLProxyHTTP, kCBLProxyHTTPS},
    slice::{from_str, self},
};

/** Types of proxy servers, for CBLProxySettings. */
#[derive(Debug, PartialEq, Eq)]
pub enum ProxyType {
    HTTP,
    HTTPS,
}

impl From<CBLProxyType> for ProxyType {
    fn from(proxy_type: CBLProxyType) -> Self {
        match u32::from(proxy_type) {
            kCBLProxyHTTP => Self::HTTP,
            kCBLProxyHTTPS => Self::HTTPS,
            _ => unreachable!(),
        }
    }
}
impl From<ProxyType> for CBLProxyType {
    fn from(proxy_type: ProxyType) -> Self {
        match proxy_type {
            ProxyType::HTTP => kCBLProxyHTTP as Self,
            ProxyType::HTTPS => kCBLProxyHTTPS as Self,
        }
    }
}

/** Proxy settings for the replicator. */
#[derive(Debug)]
pub struct ProxySettings {
    pub hostname: Option<String>, // Proxy server hostname or IP address
    pub username: Option<String>, // Username for proxy auth
    pub password: Option<String>, // Password for proxy auth
    cbl: CBLProxySettings,
}

impl ProxySettings {
    pub fn new(
        proxy_type: ProxyType,
        hostname: Option<String>,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let cbl = CBLProxySettings {
            type_: proxy_type.into(),
            hostname: hostname
                .as_ref()
                .map_or(slice::NULL_SLICE, |s| from_str(s).get_ref()),
            port,
            username: username
                .as_ref()
                .map_or(slice::NULL_SLICE, |s| from_str(s).get_ref()),
            password: password
                .as_ref()
                .map_or(slice::NULL_SLICE, |s| from_str(s).get_ref()),
        };

        Self {
            hostname,
            username,
            password,
            cbl,
        }
    }
}

impl CblRef for ProxySettings {
    type Output = *const CBLProxySettings;
    fn get_ref(&self) -> Self::Output {
        std::ptr::addr_of!(self.cbl)
    }
}
