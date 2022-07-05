
extern crate couchbase_lite;
extern crate tempdir;

use self::couchbase_lite::*;
use self::tempdir::TempDir;

use std::{ path::Path, ptr, sync::mpsc };

// Enables check for leaks of native CBL objects after `with_db()` finishes.
// WARNING: These checks only work if one test method runs at a time, i.e. testing is single
//          threaded. Run as `cargo test -- --test-threads=1` or you'll get false positives.
const LEAK_CHECKS : bool = true;

pub const DB_NAME : &str = "test_db";

const LEVEL_PREFIX : [&str;5] = ["((", "_", "", "WARNING: ", "***ERROR: "];
const LEVEL_SUFFIX : [&str;5] = ["))", "_", "", "",          " ***"];

fn logger(domain: logging::Domain, level: logging::Level, message: &str) {
    // Log to stdout, not stderr, so that `cargo test` will buffer the output.
    let i = level as usize;
    println!("CBL {:?}: {}{}{}",
             domain, LEVEL_PREFIX[i], message, LEVEL_SUFFIX[i])

}

fn init_logging() {
    logging::set_callback(Some(logger));
    logging::set_callback_level(logging::Level::Verbose);
    logging::set_console_level(logging::Level::None);
}

// Test wrapper function -- takes care of creating and deleting the database.
pub fn with_db<F>(f: F)
    where F: Fn(&mut Database)
{
    init_logging();

    let start_inst_count = instance_count() as isize;
    let tmp_dir = TempDir::new("cbl_rust").expect("create temp dir");
    let cfg = DatabaseConfiguration{
        directory: tmp_dir.path(),
        encryption_key: ptr::null_mut(),
    };
    let mut db = Database::open(DB_NAME, Some(cfg)).expect("open db");
    assert!(Database::exists(DB_NAME, tmp_dir.path()));

    f(&mut db);

    drop(db);
    if LEAK_CHECKS && instance_count() as isize > start_inst_count {
        warn!("Couchbase Lite objects were leaked by this test");
        dump_instances();
        panic!("Native object leak: {} objects, was {}",
            instance_count(), start_inst_count);
        // NOTE: This failure is likely to happen if the tests run multi-threaded, as happens by
        // default. Looking for changes in the `instance_count()` is intrinsically not thread safe.
        // Either run tests with `cargo test -- --test-threads`, or turn off `LEAK_CHECKS`.
    }
}

pub fn add_doc(db: &mut Database, id: &str, i: i64, s: &str) {
    let mut doc = Document::new_with_id(id);
    let mut props = doc.mutable_properties();
    props.at("i").put_i64(i);
    props.at("s").put_string(s);
    db.save_document(&mut doc, ConcurrencyControl::FailOnConflict).expect("save");
}

pub fn close_db(db_thread: std::thread::JoinHandle<()>, db_exec: DbQueryExecutor) {
    drop(db_exec);
    db_thread.join().expect("Couldn't join on the DB thread");
}

pub fn delete_db(db_path: &Path) {
    match Database::open(db_path.clone().to_str().unwrap(), None) {
        Ok(db) => {
            db.delete().unwrap();
        }
        Err(err) => {
            println!("Error: Initialiazion cause error: {}", err);
        }
    };
}

type Job<T> = Box<dyn FnOnce(&mut Option<T>) + Send>;

#[derive(Clone)]
pub struct DbQueryExecutor {
    inner: mpsc::Sender<Job<Database>>,
}

impl DbQueryExecutor {
    pub fn spawn<F: FnOnce(&mut Option<Database>) + Send + 'static>(&self, job: F) {
        self.inner
            .send(Box::new(job))
            .expect("thread_pool::Executor::spawn failed");
    }
}

pub fn run_db_thread(db_path: &Path) -> (std::thread::JoinHandle<()>, DbQueryExecutor) {
    let (sender, receiver) = std::sync::mpsc::channel::<Job<Database>>();
    let db_path: std::path::PathBuf = db_path.into();
    let join_handle = std::thread::spawn(move || {
        let mut db = match Database::open(db_path.as_path().to_str().unwrap(), None) {
            Ok(db) => {
                Some(db)
            }
            Err(err) => {
                println!("Error: Initialiazion cause error: {}", err);
                None
            }
        };
        loop {
            match receiver.recv() {
                Ok(x) => x(&mut db),
                Err(err) => {
                    println!("Error: db_thread recv error: {}", err);
                    break;
                }
            }
        }
    });
    (join_handle, DbQueryExecutor { inner: sender })
}
