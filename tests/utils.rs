extern crate couchbase_lite;
extern crate tempdir;

use self::couchbase_lite::*;
use self::tempdir::TempDir;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    thread, time,
};

// Enables check for leaks of native CBL objects after `with_db()` finishes.
// WARNING: These checks only work if one test method runs at a time, i.e. testing is single
//          threaded. Run as `cargo test -- --test-threads=1` or you'll get false positives.
const LEAK_CHECK: Option<&'static str> = option_env!("LEAK_CHECK");

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

pub fn init_logging() {
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
        encryption_key: None,
    };
    let mut db = Database::open(DB_NAME, Some(cfg)).expect("open db");
    assert!(Database::exists(DB_NAME, tmp_dir.path()));

    f(&mut db);

    db.delete().unwrap();

    if LEAK_CHECK.is_some() {
        warn!("Couchbase Lite objects were leaked by this test");
        dump_instances();
        assert_eq!(
            instance_count() as usize,
            start_inst_count as usize,
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

#[derive(Clone)]
pub struct ReplicationTestConfiguration {
    pub replicator_type: ReplicatorType,
    pub continuous: bool,
    pub document_ids: MutableArray,
}

impl Default for ReplicationTestConfiguration {
    fn default() -> Self {
        Self {
            replicator_type: ReplicatorType::PushAndPull,
            continuous: true,
            document_ids: MutableArray::default(),
        }
    }
}

fn generate_replication_configuration(
    local_db: &Database,
    central_db: &Database,
    config: ReplicationTestConfiguration,
) -> ReplicatorConfiguration {
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
        channels: MutableArray::default(),
        document_ids: config.document_ids,
        accept_parent_domain_cookies: false,
    }
}

pub struct ReplicationTwoDbsTester {
    _tmp_dir: TempDir,
    pub local_database: Database,
    central_database: Database,
    replicator: Replicator,
    replicator_continuous: bool,
}

impl ReplicationTwoDbsTester {
    pub fn new(
        replication_configuration: ReplicationTestConfiguration,
        context: Box<ReplicationConfigurationContext>,
    ) -> Self {
        init_logging();

        // Create databases
        let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
        let tmp_dir_path = tmp_dir.path();
        let local_database_configuration = DatabaseConfiguration {
            directory: tmp_dir_path,
            encryption_key: None,
        };
        let central_database_configuration = DatabaseConfiguration {
            directory: tmp_dir_path,
            encryption_key: None,
        };

        let local_database =
            Database::open("local", Some(local_database_configuration)).expect("open db local");
        assert!(Database::exists("local", tmp_dir_path));
        let central_database = Database::open("central", Some(central_database_configuration))
            .expect("open db central");
        assert!(Database::exists("central", tmp_dir_path));

        let replicator_continuous = replication_configuration.continuous;

        // Create replicator
        let replication_configuration = generate_replication_configuration(
            &local_database,
            &central_database,
            replication_configuration,
        );
        let mut replicator = Replicator::new(replication_configuration, context).unwrap();

        // Start replicator if needed
        if replicator_continuous {
            replicator.start(false);
        }

        // Return
        Self {
            _tmp_dir: tmp_dir,
            local_database,
            central_database,
            replicator,
            replicator_continuous,
        }
    }

    pub fn test<F>(&mut self, f: F)
    where
        F: Fn(&mut Database, &mut Database, &mut Replicator),
    {
        f(
            &mut self.local_database,
            &mut self.central_database,
            &mut self.replicator,
        );
    }

    pub fn start_replicator(&mut self) {
        self.replicator.start(false);
    }

    pub fn stop_replicator(&mut self) {
        if self.replicator_continuous {
            assert!(self.replicator.stop(None));
        }
    }

    fn new_replicator(
        &mut self,
        new_configuration: ReplicationTestConfiguration,
        new_context: Box<ReplicationConfigurationContext>,
    ) -> Replicator {
        let replicator_continuous = new_configuration.continuous;

        let new_configuration = generate_replication_configuration(
            &self.local_database,
            &self.central_database,
            new_configuration,
        );
        let mut new_replicator = Replicator::new(new_configuration, new_context).unwrap();

        if replicator_continuous {
            new_replicator.start(false);
        }

        new_replicator
    }
    pub fn change_replicator(
        &mut self,
        new_configuration: ReplicationTestConfiguration,
        new_context: Box<ReplicationConfigurationContext>,
    ) {
        self.stop_replicator();
        self.replicator_continuous = new_configuration.continuous;
        self.replicator = self.new_replicator(new_configuration, new_context);
    }
}

impl Drop for ReplicationTwoDbsTester {
    fn drop(&mut self) {
        self.stop_replicator();

        self.local_database.clone().delete().unwrap();
        self.central_database.clone().delete().unwrap();
    }
}

pub struct ReplicationThreeDbsTester {
    _tmp_dir: TempDir,
    local_database_1: Database,
    local_database_2: Database,
    central_database: Database,
    replicator_1: Replicator,
    replicator_1_continuous: bool,
    replicator_2: Replicator,
    replicator_2_continuous: bool,
}

impl ReplicationThreeDbsTester {
    pub fn new(
        replication_configuration_1: ReplicationTestConfiguration,
        replication_configuration_2: ReplicationTestConfiguration,
        context_1: Box<ReplicationConfigurationContext>,
        context_2: Box<ReplicationConfigurationContext>,
    ) -> Self {
        init_logging();

        // Create databases
        let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
        let local_database_1_configuration = DatabaseConfiguration {
            directory: tmp_dir.path(),
            encryption_key: None,
        };
        let local_database_2_configuration = DatabaseConfiguration {
            directory: tmp_dir.path(),
            encryption_key: None,
        };
        let central_database_configuration = DatabaseConfiguration {
            directory: tmp_dir.path(),
            encryption_key: None,
        };

        let local_database_1 =
            Database::open("local1", Some(local_database_1_configuration)).expect("open db local1");
        assert!(Database::exists("local1", tmp_dir.path()));
        let local_database_2 =
            Database::open("local2", Some(local_database_2_configuration)).expect("open db local2");
        assert!(Database::exists("local2", tmp_dir.path()));
        let central_database = Database::open("central", Some(central_database_configuration))
            .expect("open db central");
        assert!(Database::exists("central", tmp_dir.path()));

        let replicator_1_continuous = replication_configuration_1.continuous;
        let replicator_2_continuous = replication_configuration_2.continuous;

        // Create replicators
        let replication_configuration_1 = generate_replication_configuration(
            &local_database_1,
            &central_database,
            replication_configuration_1,
        );
        let mut replicator_1 = Replicator::new(replication_configuration_1, context_1).unwrap();

        let replication_configuration_2 = generate_replication_configuration(
            &local_database_2,
            &central_database,
            replication_configuration_2,
        );
        let mut replicator_2 = Replicator::new(replication_configuration_2, context_2).unwrap();

        // Start replicators if needed
        if replicator_1_continuous {
            replicator_1.start(false);
        }
        if replicator_2_continuous {
            replicator_2.start(false);
        }

        // Return
        Self {
            _tmp_dir: tmp_dir,
            local_database_1,
            local_database_2,
            central_database,
            replicator_1,
            replicator_1_continuous,
            replicator_2,
            replicator_2_continuous,
        }
    }

    pub fn test<F>(&mut self, f: F)
    where
        F: Fn(&mut Database, &mut Database, &mut Database, &mut Replicator, &mut Replicator),
    {
        f(
            &mut self.local_database_1,
            &mut self.local_database_2,
            &mut self.central_database,
            &mut self.replicator_1,
            &mut self.replicator_2,
        );
    }

    pub fn start_replicator_1(&mut self) {
        self.replicator_1.start(false);
    }
    pub fn start_replicator_2(&mut self) {
        self.replicator_2.start(false);
    }
    pub fn start_all_replicators(&mut self) {
        self.start_replicator_1();
        self.start_replicator_2();
    }

    pub fn stop_replicator_1(&mut self) {
        if self.replicator_1_continuous {
            assert!(self.replicator_1.stop(None));
        }
    }
    pub fn stop_replicator_2(&mut self) {
        if self.replicator_2_continuous {
            assert!(self.replicator_2.stop(None));
        }
    }
    pub fn stop_replicators(&mut self) {
        self.stop_replicator_1();
        self.stop_replicator_2();
    }

    fn new_replicator(
        &mut self,
        local_database: Database,
        new_configuration: ReplicationTestConfiguration,
        new_context: Box<ReplicationConfigurationContext>,
    ) -> Replicator {
        let replicator_continuous = new_configuration.continuous;

        let new_configuration = generate_replication_configuration(
            &local_database,
            &self.central_database,
            new_configuration,
        );
        let mut new_replicator = Replicator::new(new_configuration, new_context).unwrap();

        if replicator_continuous {
            new_replicator.start(false);
        }

        new_replicator
    }
    pub fn change_replicator_1(
        &mut self,
        new_configuration: ReplicationTestConfiguration,
        new_context: Box<ReplicationConfigurationContext>,
    ) {
        self.stop_replicator_1();
        self.replicator_1_continuous = new_configuration.continuous;
        self.replicator_1 = self.new_replicator(
            self.local_database_1.clone(),
            new_configuration,
            new_context,
        );
    }
    pub fn change_replicator_2(
        &mut self,
        new_configuration: ReplicationTestConfiguration,
        new_context: Box<ReplicationConfigurationContext>,
    ) {
        self.stop_replicator_2();
        self.replicator_2_continuous = new_configuration.continuous;
        self.replicator_2 = self.new_replicator(
            self.local_database_2.clone(),
            new_configuration,
            new_context,
        );
    }
}

impl Drop for ReplicationThreeDbsTester {
    fn drop(&mut self) {
        self.stop_replicators();

        self.local_database_1.clone().delete().unwrap();
        self.local_database_2.clone().delete().unwrap();
        self.central_database.clone().delete().unwrap();
    }
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
