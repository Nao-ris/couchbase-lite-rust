// Couchbase Lite database API
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
    CblRef, ListenerToken, release, retain,
    slice::from_str,
    error::{Result, check_bool, failure},
    c_api::{
        CBLDatabase, CBLDatabaseConfiguration, CBLDatabaseConfiguration_Default,
        CBLDatabase_AddChangeListener, CBLDatabase_BeginTransaction,
        CBLDatabase_BufferNotifications, CBLDatabase_Count, CBLDatabase_Delete,
        CBLDatabase_EndTransaction, CBLDatabase_Name, CBLDatabase_Open, CBLDatabase_Path,
        CBLDatabase_PerformMaintenance, CBLDatabase_SendNotifications, CBLEncryptionKey, CBLError,
        CBL_DatabaseExists, CBL_DeleteDatabase, FLString, kCBLMaintenanceTypeCompact,
        kCBLMaintenanceTypeFullOptimize, kCBLMaintenanceTypeIntegrityCheck,
        kCBLMaintenanceTypeOptimize, kCBLMaintenanceTypeReindex,
    },
};

use std::path::{Path, PathBuf};
use std::ptr;

/** Database configuration options. */
pub struct DatabaseConfiguration<'a> {
    pub directory: &'a std::path::Path,
    pub encryption_key: *mut CBLEncryptionKey,
}

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum MaintenanceType {
        Compact         = kCBLMaintenanceTypeCompact as isize,
        Reindex         = kCBLMaintenanceTypeReindex as isize,
        IntegrityCheck  = kCBLMaintenanceTypeIntegrityCheck as isize,
        Optimize        = kCBLMaintenanceTypeOptimize as isize,
        FullOptimize    = kCBLMaintenanceTypeFullOptimize as isize,
    }
}

type ChangeListener = fn(db: &Database, doc_ids: Vec<String>);
#[no_mangle]
unsafe extern "C" fn c_database_change_listener(
    context: *mut ::std::os::raw::c_void,
    db: *const CBLDatabase,
    num_docs: ::std::os::raw::c_uint,
    c_doc_ids: *mut FLString,
) {
    let callback: ChangeListener = std::mem::transmute(context);

    let database = Database::retain(db as *mut CBLDatabase);

    let doc_ids = std::slice::from_raw_parts(c_doc_ids, num_docs as usize)
        .iter()
        .filter_map(|doc_id| doc_id.to_string())
        .collect();

    callback(&database, doc_ids);
}

type BufferNotifications = fn(db: &Database);
#[no_mangle]
unsafe extern "C" fn c_database_buffer_notifications(
    context: *mut ::std::os::raw::c_void,
    db: *mut CBLDatabase,
) {
    let callback: BufferNotifications = std::mem::transmute(context);

    let database = Database::retain(db.cast::<CBLDatabase>());

    callback(&database);
}

/** A connection to an open database. */
#[derive(Debug, PartialEq, Eq)]
pub struct Database {
    cbl_ref: *mut CBLDatabase,
}

impl CblRef for Database {
    type Output = *mut CBLDatabase;
    fn get_ref(&self) -> Self::Output {
        self.cbl_ref
    }
}

impl Database {
    //////// CONSTRUCTORS:
    pub(crate) fn retain(cbl_ref: *mut CBLDatabase) -> Self {
        Self {
            cbl_ref: unsafe { retain(cbl_ref) },
        }
    }

    pub(crate) const fn wrap(cbl_ref: *mut CBLDatabase) -> Self {
        Self { cbl_ref }
    }

    /** Opens a database, or creates it if it doesn't exist yet, returning a new `Database`
    instance.

    It's OK to open the same database file multiple times. Each `Database` instance is
    independent of the others (and must be separately closed and released.) */
    pub fn open(name: &str, config: Option<DatabaseConfiguration>) -> Result<Self> {
        unsafe {
            if let Some(cfg) = config {
                let mut c_config: CBLDatabaseConfiguration = CBLDatabaseConfiguration_Default();
                c_config.directory = from_str(cfg.directory.to_str().unwrap()).get_ref();
                if let Some(encryption_key) = cfg.encryption_key.as_ref() {
                    c_config.encryptionKey = *encryption_key;
                }
                return Self::_open(name, &c_config);
            }
            Self::_open(name, ptr::null())
        }
    }

    unsafe fn _open(name: &str, config_ptr: *const CBLDatabaseConfiguration) -> Result<Self> {
        let mut err = CBLError::default();
        let db_ref = CBLDatabase_Open(from_str(name).get_ref(), config_ptr, &mut err);
        if db_ref.is_null() {
            return failure(err);
        }
        Ok(Self::wrap(db_ref))
    }

    //////// OTHER STATIC METHODS:

    /** Returns true if a database with the given name exists in the given directory. */
    pub fn exists<P: AsRef<Path>>(name: &str, in_directory: P) -> bool {
        unsafe {
            CBL_DatabaseExists(
                from_str(name).get_ref(),
                from_str(in_directory.as_ref().to_str().unwrap()).get_ref(),
            )
        }
    }

    /** Deletes a database file. If the database file is open, an error is returned. */
    pub fn delete_file<P: AsRef<Path>>(name: &str, in_directory: P) -> Result<bool> {
        unsafe {
            let mut error = CBLError::default();
            if CBL_DeleteDatabase(
                from_str(name).get_ref(),
                from_str(in_directory.as_ref().to_str().unwrap()).get_ref(),
                &mut error,
            ) {
                Ok(true)
            } else if !error {
                Ok(false)
            } else {
                failure(error)
            }
        }
    }

    //////// OPERATIONS:

    /** Closes and deletes a database. If there are any other connections to the database,
    an error is returned. */
    pub fn delete(self) -> Result<()> {
        unsafe { check_bool(|error| CBLDatabase_Delete(self.get_ref(), error)) }
    }

    /** Compacts a database file, freeing up unused disk space. */
    pub fn perform_maintenance(&mut self, of_type: MaintenanceType) -> Result<()> {
        unsafe {
            check_bool(|error| {
                CBLDatabase_PerformMaintenance(self.get_ref(), of_type as u32, error)
            })
        }
    }

    /** Invokes the callback within a database transaction
    - Multiple writes are _much_ faster when grouped in a transaction.
    - Changes will not be visible to other Database instances on the same database until
           the transaction ends.
    - Transactions can nest. Changes are not committed until the outer one ends. */
    pub fn in_transaction<T>(&mut self, callback: fn(&mut Self) -> Result<T>) -> Result<T> {
        let mut err = CBLError::default();
        unsafe {
            if !CBLDatabase_BeginTransaction(self.get_ref(), &mut err) {
                return failure(err);
            }
        }
        let result = callback(self);
        unsafe {
            if !CBLDatabase_EndTransaction(self.get_ref(), result.is_ok(), &mut err) {
                return failure(err);
            }
        }
        result
    }

    //////// ACCESSORS:

    /** Returns the database's name. */
    pub fn name(&self) -> &str {
        unsafe { CBLDatabase_Name(self.get_ref()).as_str().unwrap() }
    }

    /** Returns the database's full filesystem path. */
    pub fn path(&self) -> PathBuf {
        unsafe { PathBuf::from(CBLDatabase_Path(self.get_ref()).to_string().unwrap()) }
    }

    /** Returns the number of documents in the database. */
    pub fn count(&self) -> u64 {
        unsafe { CBLDatabase_Count(self.get_ref()) }
    }

    //////// NOTIFICATIONS:

    /** Registers a database change listener function. It will be called after one or more
    documents are changed on disk. Remember to keep the reference to the ChangeListener
    if you want the callback to keep working. */
    pub fn add_listener(&mut self, listener: ChangeListener) -> ListenerToken {
        unsafe {
            let callback = listener as *mut std::ffi::c_void;

            ListenerToken::new(CBLDatabase_AddChangeListener(
                self.get_ref(),
                Some(c_database_change_listener),
                callback,
            ))
        }
    }

    /** Switches the database to buffered-notification mode. Notifications for objects belonging
    to this database (documents, queries, replicators, and of course the database) will not be
    called immediately; your callback function will be called instead. You can then call
    `send_notifications` when you're ready. */
    pub fn buffer_notifications(&self, callback: BufferNotifications) {
        unsafe {
            let callback = callback as *mut std::ffi::c_void;

            CBLDatabase_BufferNotifications(
                self.get_ref(),
                Some(c_database_buffer_notifications),
                callback,
            );
        }
    }

    /** Immediately issues all pending notifications for this database, by calling their listener
    callbacks. (Only useful after `buffer_notifications` has been called.) */
    pub fn send_notifications(&self) {
        unsafe {
            CBLDatabase_SendNotifications(self.get_ref());
        }
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        unsafe { release(self.get_ref()) }
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self::retain(self.get_ref())
    }
}
