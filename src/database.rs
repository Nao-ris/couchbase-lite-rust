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
        CBLDatabase_BufferNotifications, CBLDatabase_ChangeEncryptionKey, CBLDatabase_Close,
        CBLDatabase_Count, CBLDatabase_Delete, CBLDatabase_EndTransaction, CBLDatabase_Name,
        CBLDatabase_Open, CBLDatabase_Path, CBLDatabase_PerformMaintenance,
        CBLDatabase_SendNotifications, CBLEncryptionKey, CBLError, CBL_DatabaseExists,
        CBL_DeleteDatabase, CBLEncryptionKey_FromPassword, FLString, kCBLMaintenanceTypeCompact,
        kCBLEncryptionNone, kCBLMaintenanceTypeFullOptimize, kCBLMaintenanceTypeIntegrityCheck,
        kCBLMaintenanceTypeOptimize, kCBLMaintenanceTypeReindex, CBL_CopyDatabase,
    },
    Listener, check_error, Error, CouchbaseLiteError,
};
use std::path::{Path, PathBuf};
use std::ptr;

#[derive(Debug, Clone)]
pub struct EncryptionKey {
    cbl_ref: Box<CBLEncryptionKey>,
}

impl EncryptionKey {
    pub fn new_from_password(password: &str) -> Option<Self> {
        unsafe {
            let key = CBLEncryptionKey {
                algorithm: kCBLEncryptionNone,
                bytes: [0; 32],
            };
            let encryption_key = Self {
                cbl_ref: Box::new(key),
            };

            if CBLEncryptionKey_FromPassword(
                encryption_key.get_ref() as *mut CBLEncryptionKey,
                from_str(password).get_ref(),
            ) {
                Some(encryption_key)
            } else {
                None
            }
        }
    }
}

impl CblRef for EncryptionKey {
    type Output = *const CBLEncryptionKey;
    fn get_ref(&self) -> Self::Output {
        std::ptr::addr_of!(*self.cbl_ref)
    }
}

/** Database configuration options. */
#[derive(Debug, Clone)]
pub struct DatabaseConfiguration<'a> {
    pub directory: &'a std::path::Path,
    pub encryption_key: Option<EncryptionKey>,
}

enum_from_primitive! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MaintenanceType {
        Compact         = kCBLMaintenanceTypeCompact as isize,
        Reindex         = kCBLMaintenanceTypeReindex as isize,
        IntegrityCheck  = kCBLMaintenanceTypeIntegrityCheck as isize,
        Optimize        = kCBLMaintenanceTypeOptimize as isize,
        FullOptimize    = kCBLMaintenanceTypeFullOptimize as isize,
    }
}

/** A database change listener callback, invoked after one or more documents are changed on disk. */
type ChangeListener = Box<dyn Fn(&Database, Vec<String>)>;

#[no_mangle]
unsafe extern "C" fn c_database_change_listener(
    context: *mut ::std::os::raw::c_void,
    db: *const CBLDatabase,
    num_docs: ::std::os::raw::c_uint,
    c_doc_ids: *mut FLString,
) {
    let callback = context as *const ChangeListener;
    let database = Database::retain(db as *mut CBLDatabase);

    let doc_ids = std::slice::from_raw_parts(c_doc_ids, num_docs as usize)
        .iter()
        .filter_map(|doc_id| doc_id.to_string())
        .collect();

    (*callback)(&database, doc_ids);
}

/** Callback indicating that the database (or an object belonging to it) is ready to call one or more listeners. */
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
                if let Some(encryption_key) = cfg.encryption_key {
                    c_config.encryptionKey = *encryption_key.get_ref();
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

    /** Copies a database file to a new location, and assigns it a new internal UUID to distinguish
    it from the original database when replicating. */
    pub fn copy_file<P: AsRef<Path>>(
        current_db_full_path: P,
        new_db_name: &str,
        config: Option<DatabaseConfiguration>,
    ) -> Result<()> {
        let config = config
            .map(|cfg| {
                let mut c_config: CBLDatabaseConfiguration =
                    unsafe { CBLDatabaseConfiguration_Default() };

                c_config.directory = from_str(
                    cfg.directory
                        .to_str()
                        .ok_or(Error::cbl_error(CouchbaseLiteError::InvalidParameter))?,
                )
                .get_ref();

                if let Some(encryption_key) = cfg.encryption_key {
                    c_config.encryptionKey = unsafe { *encryption_key.get_ref() };
                }
                Ok(c_config)
            })
            .transpose()?;

        let config_param = config
            .as_ref()
            .map(|cfg| cfg as *const CBLDatabaseConfiguration)
            .unwrap_or(ptr::null());

        let mut error = CBLError::default();
        unsafe {
            CBL_CopyDatabase(
                from_str(
                    current_db_full_path
                        .as_ref()
                        .to_str()
                        .ok_or(Error::cbl_error(CouchbaseLiteError::InvalidParameter))?,
                )
                .get_ref(),
                from_str(new_db_name).get_ref(),
                config_param,
                &mut error,
            );
        }

        check_error(&error)
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

    /** Closes an open database. */
    pub fn close(self) -> Result<()> {
        unsafe { check_bool(|error| CBLDatabase_Close(self.get_ref(), error)) }
    }

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
    pub fn in_transaction<T, F>(&mut self, mut callback: F) -> Result<T>
    where
        F: FnMut(&mut Self) -> Result<T>,
    {
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

    /** Encrypts or decrypts a database, or changes its encryption key. */
    pub fn change_encryption_key(&mut self, encryption_key: &EncryptionKey) -> Result<()> {
        unsafe {
            check_bool(|error| {
                CBLDatabase_ChangeEncryptionKey(self.get_ref(), encryption_key.get_ref(), error)
            })
        }
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
        if you want the callback to keep working.

        # Lifetime

        The listener is deleted at the end of life of the `Listener` object.
        You must keep the `Listener` object as long as you need it.
    */
    #[must_use]
    pub fn add_listener(&mut self, listener: ChangeListener) -> Listener<ChangeListener> {
        unsafe {
            let listener = Box::new(listener);
            let ptr = Box::into_raw(listener);

            Listener::new(
                ListenerToken {
                    cbl_ref: CBLDatabase_AddChangeListener(
                        self.cbl_ref,
                        Some(c_database_change_listener),
                        ptr.cast(),
                    ),
                },
                Box::from_raw(ptr),
            )
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
