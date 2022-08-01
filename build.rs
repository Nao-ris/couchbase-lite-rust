// Couchbase Lite C API bindings generator
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

// This script runs during a Cargo build and generates the raw/unsafe Rust bindings, "bindings.rs",
// in an internal build directory, where they are included by `src/c_api.rs`.
//
// References:
// - https://rust-lang.github.io/rust-bindgen/tutorial-3.html
// - https://doc.rust-lang.org/cargo/reference/build-scripts.html

extern crate bindgen;

use std::error::Error;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// Where to find the Couchbase Lite headers and library:    //TODO: Make this easily configurable
static CBL_INCLUDE_DIR: &str = "libcblite-3.0.1/include";
static CBL_LIB_DIR: &str = "libcblite-3.0.1/lib";

#[cfg(target_os = "macos")]
static CBL_LIB_FILENAME: &str = "libcblite.dylib";

#[cfg(target_os = "linux")]
static CBL_LIB_FILENAME: &str = "libcblite.so";

// Where to find Clang and LLVM libraries:
#[cfg(target_os = "macos")]
static DEFAULT_LIBCLANG_PATH: &str = "/usr/local/Cellar/llvm/12.0.1/lib";

#[cfg(target_os = "linux")]
static DEFAULT_LIBCLANG_PATH: &str = "/usr/lib/";

static STATIC_LINK_CBL: bool = false;
static CBL_SRC_DIR: &str = "../../CBL_C";

fn main() -> Result<(), Box<dyn Error>> {
    // Set LIBCLANG_PATH environment variable if it's not already set:
    if env::var("LIBCLANG_PATH").is_err() {
        env::set_var("LIBCLANG_PATH", DEFAULT_LIBCLANG_PATH);
        println!("cargo:rustc-env=LIBCLANG_PATH={}", DEFAULT_LIBCLANG_PATH);
    }

    let sdk_root = "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX12.3.sdk";

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate bindings for.
        .header("src/wrapper.h")
        //.header("libcblite-3.0.1/include/cbl/CouchbaseLite.h")
        // C '#include' search paths:
        .clang_arg(format!("-I{}", CBL_INCLUDE_DIR))
        // Which symbols to generate bindings for:
        .whitelist_type("CBL.*")
        .whitelist_type("FL.*")
        .whitelist_var("k?CBL.*")
        .whitelist_var("k?FL.*")
        .whitelist_function("CBL.*")
        .whitelist_function("_?FL.*")
        .no_copy("FLSliceResult")
        .clang_arg(format!("-isysroot{}", sdk_root))
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Tell cargo to tell rustc to link the cblite library.
    if STATIC_LINK_CBL {
        // Link against the CBL-C and LiteCore static libraries for maximal efficienty.
        // This assumes that a checkout of couchbase-lite-C exists at CBL_SRC_DIR
        // and has been built with CMake.
        // TODO: Currently, on Mac OS this requires manual processing of the static libraries
        //       because Rust can't link with fat libraries. Thus all the libraries linked below
        //       had to be thinned with e.g. `libXXX.a -thin x86_64 -output libXXX-x86.a`.
        let cblite_src_path = PathBuf::from(CBL_SRC_DIR);
        let root = cblite_src_path.to_str().unwrap();

        println!("cargo:rustc-link-search={}/build_cmake", root);
        println!(
            "cargo:rustc-link-search={}/build_cmake/vendor/couchbase-lite-core",
            root
        );
        println!(
            "cargo:rustc-link-search={}/build_cmake/vendor/couchbase-lite-core/Networking/BLIP",
            root
        );
        println!(
            "cargo:rustc-link-search={}/build_cmake/vendor/couchbase-lite-core/vendor/fleece",
            root
        );
        println!("cargo:rustc-link-search={}/build_cmake/vendor/couchbase-lite-core/vendor/mbedtls/library", root);
        println!("cargo:rustc-link-search={}/build_cmake/vendor/couchbase-lite-core/vendor/mbedtls/crypto/library", root);
        println!("cargo:rustc-link-search={}/build_cmake/vendor/couchbase-lite-core/vendor/sqlite3-unicodesn", root);

        println!("cargo:rustc-link-lib=static=cblite-static-x86");
        println!("cargo:rustc-link-lib=static=liteCoreStatic-x86");
        println!("cargo:rustc-link-lib=static=liteCoreWebSocket-x86");
        println!("cargo:rustc-link-lib=static=BLIPStatic-x86");
        println!("cargo:rustc-link-lib=static=FleeceStatic-x86");
        println!("cargo:rustc-link-lib=static=CouchbaseSqlite3-x86");
        println!("cargo:rustc-link-lib=static=SQLite3_UnicodeSN-x86");
        println!("cargo:rustc-link-lib=static=mbedcrypto-x86");
        println!("cargo:rustc-link-lib=static=mbedtls-x86");
        println!("cargo:rustc-link-lib=static=mbedx509-x86");

        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-lib=z");

        // TODO: This only applies to Apple platforms:
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=CFNetwork");
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=SystemConfiguration");
    } else {
        // Link against and copy the CBL dynamic library:
        let src = PathBuf::from(CBL_LIB_DIR).join(CBL_LIB_FILENAME);
        let dst = out_dir.join(CBL_LIB_FILENAME);
        println!("cargo:rerun-if-changed={}", src.to_str().unwrap());
        fs::copy(src, dst).expect("copy dylib");
        // Tell rustc to link it:
        println!("cargo:rustc-link-search={}", out_dir.to_str().unwrap());
        println!("cargo:rustc-link-lib=dylib=cblite");
    }

    if cfg!(target_os = "linux") {
        let dir = env!("CARGO_MANIFEST_DIR");
        println!(
            "cargo:rustc-link-search=all={}",
            std::path::Path::new(&dir)
                .join("libcblite-3.0.1/lib")
                .display()
        );

        println!(
            "cargo:rustc-env=LD_LIBRARY_PATH={}",
            std::path::Path::new(&dir)
                .join("libcblite-3.0.1/lib")
                .display()
        );

        let build_type = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };

        let lib_path = Path::new("libcblite-3.0.1/lib/");
        let dest_path = PathBuf::from(format!("target/{}/deps/", build_type));
        
        panic!("curr: {:?} out: {:?} lib: {:?}  dest: {:?}", std::env::current_dir(), std::env::var("OUT_DIR"), lib_path, dest_path);
        std::fs::copy(
            lib_path.join("libcblite.so"),
            dest_path.join("libcblite.so"),
        )?;
        std::fs::copy(
            lib_path.join("libcblite.so.3"),
            dest_path.join("libcblite.so.3"),
        )?;
        std::fs::copy(
            lib_path.join("libcblite.so.3.0.1"),
            dest_path.join("libcblite.so.3.0.1"),
        )?;
        std::fs::copy(
            lib_path.join("libcblite.so.sym"),
            dest_path.join("libcblite.so.sym"),
        )?;
        std::fs::copy(
            lib_path.join("libicudata.so.63"),
            dest_path.join("libicudata.so.63"),
        )?;
        std::fs::copy(
            lib_path.join("libicudata.so.63.1"),
            dest_path.join("libicudata.so.63.1"),
        )?;
        std::fs::copy(
            lib_path.join("libicui18n.so.63"),
            dest_path.join("libicui18n.so.63"),
        )?;
        std::fs::copy(
            lib_path.join("libicui18n.so.63.1"),
            dest_path.join("libicui18n.so.63.1"),
        )?;
        std::fs::copy(
            lib_path.join("libicuio.so.63"),
            dest_path.join("libicuio.so.63"),
        )?;
        std::fs::copy(
            lib_path.join("libicuio.so.63.1"),
            dest_path.join("libicuio.so.63.1"),
        )?;
        std::fs::copy(
            lib_path.join("libicutest.so.63"),
            dest_path.join("libicutest.so.63"),
        )?;
        std::fs::copy(
            lib_path.join("libicutest.so.63.1"),
            dest_path.join("libicutest.so.63.1"),
        )?;
        std::fs::copy(
            lib_path.join("libicutu.so.63"),
            dest_path.join("libicutu.so.63"),
        )?;
        std::fs::copy(
            lib_path.join("libicutu.so.63.1"),
            dest_path.join("libicutu.so.63.1"),
        )?;
        std::fs::copy(
            lib_path.join("libicuuc.so.63"),
            dest_path.join("libicuuc.so.63"),
        )?;
        std::fs::copy(
            lib_path.join("libicuuc.so.63.1"),
            dest_path.join("libicuuc.so.63.1"),
        )?;
    }

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=src/wrapper.h");

    Ok(())
}
