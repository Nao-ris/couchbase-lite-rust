// Couchbase Lite logging API
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

use crate::c_api::{
    CBLLogDomain, CBLLogLevel, CBLLog_SetCallback, CBLLog_SetCallbackLevel, CBLLog_SetConsoleLevel,
    CBL_Log, FLString,
};

use enum_primitive::FromPrimitive;
use std::fmt;
use std::ffi::CString;

enum_from_primitive! {
    /** Logging domains: subsystems that generate log messages. */
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Domain {
        Database,
        Query,
        Replicator,
        Network,
        None
    }
}

enum_from_primitive! {
    /** Levels of log messages. Higher values are more important/severe.
        Each level includes the lower ones. */
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Level {
        Debug,
        Verbose,
        Info,
        Warning,
        Error,
        None
    }
}

pub type LogCallback = Option<fn(Domain, Level, &str)>;

/** Sets the detail level of console logging.
Only messages whose level is ≥ the given level will be logged to the console.
Default value is Info. */
pub fn set_console_level(level: Level) {
    unsafe { CBLLog_SetConsoleLevel(level as u8) }
}

/** Sets the detail level of logging to the registered callback (if any.)
Only messages whose level is ≥ the given level will be logged to the callback.
Default value is Info. */
pub fn set_callback_level(level: Level) {
    unsafe { CBLLog_SetCallbackLevel(level as u8) }
}

/** Registers a function that will receive log messages. */
pub fn set_callback(callback: LogCallback) {
    unsafe {
        LOG_CALLBACK = callback;
        if callback.is_some() {
            CBLLog_SetCallback(Some(invoke_log_callback));
        } else {
            CBLLog_SetCallback(None);
        }
    }
}

/** Writes a log message. */
pub fn write(domain: Domain, level: Level, message: &str) {
    unsafe {
        let cstr = CString::new(message).unwrap();
        CBL_Log(domain as u8, level as u8, cstr.as_ptr());

        // CBL_Log doesn't invoke the callback, so do it manually:
        if let Some(callback) = LOG_CALLBACK {
            //if  CBLLog_WillLogToConsole(domain as u8, level as u8) {
            callback(domain, level, message);
            //}
        }
    }
}

/** Writes a log message using the given format arguments. */
pub fn write_args(domain: Domain, level: Level, args: fmt::Arguments) {
    write(domain, level, &format!("{:?}", args));
}

//////// LOGGING MACROS:

/// A macro that writes a formatted Error-level log message.
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::logging::write_args(
        $crate::logging::Domain::Database, $crate::logging::Level::Error,
        format_args!($($arg)*)));
}

/// A macro that writes a formatted Warning-level log message.
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::logging::write_args(
        $crate::logging::Domain::Database, $crate::logging::Level::Warning,
        format_args!($($arg)*)));
}

/// A macro that writes a formatted Info-level log message.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::logging::write_args(
        $crate::logging::Domain::Database, $crate::logging::Level::Info,
        format_args!($($arg)*)));
}

/// A macro that writes a formatted Verbose-level log message.
#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => ($crate::logging::write_args(
        $crate::logging::Domain::Database, $crate::logging::Level::Verbose,
        format_args!($($arg)*)));
}

/// A macro that writes a formatted Debug-level log message.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::logging::write_args(
        $crate::logging::Domain::Database, $crate::logging::Level::Debug,
        format_args!($($arg)*)));
}

//////// INTERNALS:

static mut LOG_CALLBACK: LogCallback = None;

unsafe extern "C" fn invoke_log_callback(
    c_domain: CBLLogDomain,
    c_level: CBLLogLevel,
    msg: FLString,
) {
    if let Some(cb) = LOG_CALLBACK {
        let domain = Domain::from_u8(c_domain).unwrap_or(Domain::None);
        let level = Level::from_u8(c_level).unwrap_or(Level::None);
        cb(domain, level, msg.as_str().unwrap_or("Empty error"));
    }
}
