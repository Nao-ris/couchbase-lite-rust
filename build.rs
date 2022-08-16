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

use std::env;
use std::fs;
use std::path::PathBuf;

static CBL_INCLUDE_DIR: &str = "libcblite-3.0.1/include";
static CBL_LIB_DIR: &str = "libcblite-3.0.1/lib";

#[cfg(target_os = "macos")]
static CBL_LIB_FILENAME: &str = "libcblite.dylib";
#[cfg(target_os = "linux")]
static CBL_LIB_FILENAME: &str = "libcblite.so";
#[cfg(target_os = "win32")]
static CBL_LIB_FILENAME: &str = "cblite.ddl";
#[cfg(all(target_os = "android", target_arch = "aarch64"))]
static CBL_LIB_FILENAME: &str = "libcblite.arm64-v8a.so";
#[cfg(all(target_os = "android", target_arch = "arm"))]
static CBL_LIB_FILENAME: &str = "libcblite.armeabi-v7a.so";

fn main() {
    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
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
    // Link against and copy the CBL dynamic library:
    let src = PathBuf::from(CBL_LIB_DIR).join(CBL_LIB_FILENAME);
    let dst = out_dir.join(CBL_LIB_FILENAME);
    println!("cargo:rerun-if-changed={}", src.to_str().unwrap());
    fs::copy(src, dst).expect("copy dylib");
    // Tell rustc to link it:
    println!("cargo:rustc-link-search={}", out_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=dylib=cblite");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=src/wrapper.h");

    println!(
        "cargo:rustc-link-search={}/libcblite-3.0.1/lib",
        env!("CARGO_MANIFEST_DIR")
    );

    setup();
}

pub fn setup() {
    let lib_path = PathBuf::from(format!(
        "{}/libcblite-3.0.1/lib/",
        env!("CARGO_MANIFEST_DIR")
    ));

    let dest_path = PathBuf::from(format!("{}/", std::env::var("OUT_DIR").unwrap()));

    std::fs::copy(
        lib_path.join("libcblite.so"),
        dest_path.join("libcblite.so"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libcblite.so.3"),
        dest_path.join("libcblite.so.3"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libcblite.so.3.0.1"),
        dest_path.join("libcblite.so.3.0.1"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libcblite.so.sym"),
        dest_path.join("libcblite.so.sym"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicudata.so.63"),
        dest_path.join("libicudata.so.63"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicudata.so.63.1"),
        dest_path.join("libicudata.so.63.1"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicui18n.so.63"),
        dest_path.join("libicui18n.so.63"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicui18n.so.63.1"),
        dest_path.join("libicui18n.so.63.1"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicuio.so.63"),
        dest_path.join("libicuio.so.63"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicuio.so.63.1"),
        dest_path.join("libicuio.so.63.1"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicutest.so.63"),
        dest_path.join("libicutest.so.63"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicutest.so.63.1"),
        dest_path.join("libicutest.so.63.1"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicutu.so.63"),
        dest_path.join("libicutu.so.63"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicutu.so.63.1"),
        dest_path.join("libicutu.so.63.1"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicuuc.so.63"),
        dest_path.join("libicuuc.so.63"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libicuuc.so.63.1"),
        dest_path.join("libicuuc.so.63.1"),
    )
    .unwrap();
    std::fs::copy(lib_path.join("cblite.dll"), dest_path.join("cblite.dll")).unwrap();
    std::fs::copy(lib_path.join("cblite.lib"), dest_path.join("cblite.lib")).unwrap();
    std::fs::copy(
        lib_path.join("libcblite.arm64-v8a.so"),
        dest_path.join("libcblite.so"),
    )
    .unwrap();
    std::fs::copy(
        lib_path.join("libcblite.armeabi-v7a.so"),
        dest_path.join("libcblite.so"),
    )
    .unwrap();
}
