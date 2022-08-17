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
use std::path::PathBuf;

static CBL_INCLUDE_DIR: &str = "libcblite-3.0.1/include";
static CBL_LIB_DIR: &str = "libcblite-3.0.1/lib";

fn main() {
    generate_bindings();
    configure_rustc();
    copy_lib();
}

fn generate_bindings() {
    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .clang_arg(format!("-I{}", CBL_INCLUDE_DIR))
        .whitelist_type("CBL.*")
        .whitelist_type("FL.*")
        .whitelist_var("k?CBL.*")
        .whitelist_var("k?FL.*")
        .whitelist_function("CBL.*")
        .whitelist_function("_?FL.*")
        .no_copy("FLSliceResult")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn configure_rustc() {
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:rerun-if-changed={}", CBL_INCLUDE_DIR);
    println!("cargo:rerun-if-changed={}", CBL_LIB_DIR);
    println!(
        "cargo:rustc-link-search={}/{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        CBL_LIB_DIR,
        std::env::var("TARGET").unwrap()
    );
    println!("cargo:rustc-link-search={}", env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-link-lib=dylib=cblite");
}

pub fn copy_lib() {
    let lib_path = PathBuf::from(format!(
        "{}/{}/{}/",
        env!("CARGO_MANIFEST_DIR"),
        CBL_LIB_DIR,
        std::env::var("TARGET").unwrap()
    ));
    let dest_path = PathBuf::from(format!("{}/", std::env::var("OUT_DIR").unwrap()));

    if cfg!(target_os = "linux") || cfg!(target_os = "android") {
        std::fs::copy(
            lib_path.join("libcblite.so"),
            dest_path.join("libcblite.so"),
        )
        .unwrap();
    }

    /*
    #[cfg(all(target_os = "android", target_arch = "aarch64"))]
    std::fs::copy(
        lib_path.join("android/aarch64/libcblite.stripped.so"),
        dest_path.join("libcblite.so"),
    )
    .unwrap();
    #[cfg(all(target_os = "android", target_arch = "arm"))]
    std::fs::copy(
        lib_path.join("android/arm/libcblite.stripped.so"),
        dest_path.join("libcblite.so"),
    )
    .unwrap();

    if cfg!(target_os = "linux") {
        std::fs::copy(
            lib_path.join("linux/libcblite.so"),
            dest_path.join("libcblite.so"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libcblite.so.3"),
            dest_path.join("libcblite.so.3"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libcblite.so.3.0.1"),
            dest_path.join("libcblite.so.3.0.1"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libcblite.so.sym"),
            dest_path.join("libcblite.so.sym"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicudata.so.63"),
            dest_path.join("libicudata.so.63"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicudata.so.63.1"),
            dest_path.join("libicudata.so.63.1"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicui18n.so.63"),
            dest_path.join("libicui18n.so.63"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicui18n.so.63.1"),
            dest_path.join("libicui18n.so.63.1"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicuio.so.63"),
            dest_path.join("libicuio.so.63"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicuio.so.63.1"),
            dest_path.join("libicuio.so.63.1"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicutest.so.63"),
            dest_path.join("libicutest.so.63"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicutest.so.63.1"),
            dest_path.join("libicutest.so.63.1"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicutu.so.63"),
            dest_path.join("libicutu.so.63"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicutu.so.63.1"),
            dest_path.join("libicutu.so.63.1"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicuuc.so.63"),
            dest_path.join("libicuuc.so.63"),
        )
        .unwrap();
        std::fs::copy(
            lib_path.join("linux/libicuuc.so.63.1"),
            dest_path.join("libicuuc.so.63.1"),
        )
        .unwrap();
    }*/

    if cfg!(target_os = "macos") {
        std::fs::copy(
            lib_path.join("libcblite.dylib"),
            dest_path.join("libcblite.dylib"),
        )
        .unwrap();
    }

    if cfg!(target_os = "windows") {
        std::fs::copy(lib_path.join("cblite.dll"), dest_path.join("cblite.dll")).unwrap();
        std::fs::copy(lib_path.join("cblite.lib"), dest_path.join("cblite.lib")).unwrap();
    }
}
