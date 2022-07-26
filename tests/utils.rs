extern crate couchbase_lite;
extern crate tempdir;

use self::couchbase_lite::*;
use self::tempdir::TempDir;

use std::{
    collections::HashMap,
    ptr,
    sync::{Arc, Mutex, MutexGuard},
    thread, time,
};

// Enables check for leaks of native CBL objects after `with_db()` finishes.
// WARNING: These checks only work if one test method runs at a time, i.e. testing is single
//          threaded. Run as `cargo test -- --test-threads=1` or you'll get false positives.
const LEAK_CHECKS: bool = true;

pub const DB_NAME: &str = "test_db";

const LEVEL_PREFIX: [&str; 5] = ["((", "_", "", "WARNING: ", "***ERROR: "];
const LEVEL_SUFFIX: [&str; 5] = ["))", "_", "", "", " ***"];

fn logger(domain: logging::Domain, level: logging::Level, message: &str) {
    // Log to stdout, not stderr, so that `cargo test` will buffer the output.
    let i = level as usize;
    println!(
        "CBL {:?}: {}{}{}",
        domain, LEVEL_PREFIX[i], message, LEVEL_SUFFIX[i]
    )
}

fn init_logging() {
    logging::set_callback(Some(logger));
    logging::set_callback_level(logging::Level::Verbose);
    logging::set_console_level(logging::Level::None);
}

// Test wrapper function -- takes care of creating and deleting the database.
pub fn with_db<F>(f: F)
where
    F: Fn(&mut Database),
{
    init_logging();

    let start_inst_count = instance_count() as isize;
    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: ptr::null_mut(),
    };
    let mut db = Database::open(DB_NAME, Some(cfg)).expect("open db");
    assert!(Database::exists(DB_NAME, tmp_dir.path()));

    f(&mut db);

    db.delete().unwrap();
    if LEAK_CHECKS && instance_count() as isize > start_inst_count {
        warn!("Couchbase Lite objects were leaked by this test");
        dump_instances();
        panic!(
            "Native object leak: {} objects, was {}",
            instance_count(),
            start_inst_count
        );
        // NOTE: This failure is likely to happen if the tests run multi-threaded, as happens by
        // default. Looking for changes in the `instance_count()` is intrinsically not thread safe.
        // Either run tests with `cargo test -- --test-threads`, or turn off `LEAK_CHECKS`.
    }
}

// Replication

pub struct ReplicationTestConfiguration<'a> {
    pub replicator_type: ReplicatorType,
    pub continuous: bool,
    pub document_ids: Array<'a>,
    pub push_filter: Option<ReplicationFilter>,
    pub pull_filter: Option<ReplicationFilter>,
    pub conflict_resolver: Option<ConflictResolver>,
    pub property_encryptor: Option<PropertyEncryptor>,
    pub property_decryptor: Option<PropertyDecryptor>,
}
impl<'a> Default for ReplicationTestConfiguration<'a> {
    fn default() -> Self {
        Self {
            replicator_type: ReplicatorType::PushAndPull,
            continuous: true,
            document_ids: Array::default(),
            push_filter: None,
            pull_filter: None,
            conflict_resolver: None,
            property_encryptor: None,
            property_decryptor: None,
        }
    }
}

fn generate_replication_configuration<'a>(
    local_db: &Database,
    central_db: &Database,
    config: ReplicationTestConfiguration<'a>,
) -> ReplicatorConfiguration<'a> {
    ReplicatorConfiguration {
        database: local_db.clone(),
        endpoint: Endpoint::new_with_local_db(central_db),
        replicator_type: config.replicator_type,
        continuous: config.continuous,
        disable_auto_purge: true,
        max_attempts: 4,
        max_attempt_wait_time: 100,
        heartbeat: 120,
        authenticator: None,
        proxy: None,
        headers: HashMap::new(),
        pinned_server_certificate: None,
        trusted_root_certificates: None,
        channels: Array::default(),
        document_ids: config.document_ids,
        push_filter: config.push_filter,
        pull_filter: config.pull_filter,
        conflict_resolver: config.conflict_resolver,
        property_encryptor: config.property_encryptor,
        property_decryptor: config.property_decryptor,
    }
}

pub fn with_three_dbs<F>(
    config1: ReplicationTestConfiguration,
    config2: ReplicationTestConfiguration,
    f: F,
) where
    F: Fn(&mut Database, &mut Database, &mut Database, &mut Replicator, &mut Replicator),
{
    init_logging();

    // Create databases
    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg1 = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: ptr::null_mut(),
    };
    let cfg2 = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: ptr::null_mut(),
    };
    let cfg3 = DatabaseConfiguration {
        directory: tmp_dir.path(),
        encryption_key: ptr::null_mut(),
    };
    let mut local_db1 = Database::open("local1", Some(cfg1)).expect("open db local1");
    assert!(Database::exists("local1", tmp_dir.path()));
    let mut local_db2 = Database::open("local2", Some(cfg2)).expect("open db local2");
    assert!(Database::exists("local2", tmp_dir.path()));
    let mut central_db = Database::open("central", Some(cfg3)).expect("open db central");
    assert!(Database::exists("central", tmp_dir.path()));

    // Create replicators
    let config1 = generate_replication_configuration(&local_db1, &central_db, config1);
    let mut repl1 = Replicator::new(config1).unwrap();

    let config2 = generate_replication_configuration(&local_db2, &central_db, config2);
    let mut repl2 = Replicator::new(config2).unwrap();

    // Start replicators
    repl1.start(false);
    repl2.start(false);

    // Callback
    f(
        &mut local_db1,
        &mut local_db2,
        &mut central_db,
        &mut repl1,
        &mut repl2,
    );

    // Clean up
    repl1.stop();
    repl2.stop();

    local_db1.delete().unwrap();
    local_db2.delete().unwrap();
    central_db.delete().unwrap();
}

pub fn add_doc(db: &mut Database, id: &str, i: i64, s: &str) {
    let mut doc = Document::new_with_id(id);
    let mut props = doc.mutable_properties();
    props.at("i").put_i64(i);
    props.at("s").put_string(s);
    db.save_document_with_concurency_control(&mut doc, ConcurrencyControl::FailOnConflict)
        .expect("save");
}

// Static

pub fn get_static<T>(st: &Arc<Mutex<T>>) -> MutexGuard<T> {
    if let Ok(st) = st.lock() {
        st
    } else {
        panic!("Impossible to lock static")
    }
}
pub fn get_static_value<T>(st: &Arc<Mutex<T>>) -> T
where
    T: Copy,
{
    if let Ok(st) = st.lock() {
        *st
    } else {
        panic!("Impossible to lock static")
    }
}
pub fn set_static<T>(st: &Arc<Mutex<T>>, value: T) {
    *get_static(st) = value;
}

pub fn is_static_true(st: &Arc<Mutex<bool>>) -> bool {
    get_static_value(st)
}
pub fn check_static_with_wait<T>(
    st: &Arc<Mutex<T>>,
    expected_value: T,
    max_wait_seconds: Option<u64>,
) -> bool
where
    T: Copy + std::cmp::PartialEq,
{
    let max_wait_seconds = time::Duration::from_secs(max_wait_seconds.unwrap_or(10));
    let now = time::Instant::now();
    let wait_time = time::Duration::from_millis(100);

    let mut result = get_static_value(st) == expected_value;
    while !result && now.elapsed() < max_wait_seconds {
        thread::sleep(wait_time);
        result = get_static_value(st) == expected_value;
    }

    result
}
pub fn check_callback_with_wait<CB>(mut callback: CB, max_wait_seconds: Option<u64>) -> bool
where
    CB: FnMut() -> bool,
{
    let max_wait_seconds = time::Duration::from_secs(max_wait_seconds.unwrap_or(10));
    let now = time::Instant::now();
    let wait_time = time::Duration::from_millis(100);

    let mut result = callback();
    while !result && now.elapsed() < max_wait_seconds {
        thread::sleep(wait_time);
        result = callback();
    }

    result
}
