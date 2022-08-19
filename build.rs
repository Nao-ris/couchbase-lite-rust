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
use std::error::Error;
use std::fs;
use std::path::PathBuf;

static CBL_INCLUDE_DIR: &str = "libcblite-3.0.2/include";
static CBL_LIB_DIR: &str = "libcblite-3.0.2/lib";

fn wrapper_path() -> &'static str {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os != "ios" {
        "src/wrapper.h"
    } else {
        "src/wrapper_ios.h"
    }
}

fn headers_dir() -> &'static str {
    if env::var("TARGET").unwrap().ends_with("apple-ios") {
        "libcblite-3.0.2/lib/aarch64-apple-ios/CouchbaseLite.xcframework/ios-arm64_armv7/CouchbaseLite.framework/Headers"
    } else if env::var("TARGET").unwrap().ends_with("apple-ios-sim") {
        "libcblite-3.0.2/lib/aarch64-apple-ios/CouchbaseLite.xcframework/ios-arm64_i386_x86_64-simulator/CouchbaseLite.framework/Headers"
    } else {
        CBL_INCLUDE_DIR
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    generate_bindings()?;
    configure_rustc()?;
    copy_lib()?;

    Ok(())
}

fn generate_bindings() -> Result<(), Box<dyn Error>> {
    let bindings = bindgen::Builder::default()
        .header(wrapper_path())
        .clang_arg(format!("-I{}", headers_dir()))
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

    let out_dir = env::var("OUT_DIR")?;
    bindings
        .write_to_file(PathBuf::from(out_dir).join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Ok(())
}

fn configure_rustc() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:rerun-if-changed={}", CBL_INCLUDE_DIR);
    println!("cargo:rerun-if-changed={}", CBL_LIB_DIR);
    let target_dir = env::var("TARGET")?;
    println!(
        "cargo:rustc-link-search={}/{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        CBL_LIB_DIR,
        target_dir
    );
    println!(
        "cargo:rustc-link-search=framework={}/{}/{}/CouchbaseLite.xcframework/ios-arm64_armv7",
        env!("CARGO_MANIFEST_DIR"),
        CBL_LIB_DIR,
        target_dir
    );
    println!("cargo:rustc-link-search={}", env::var("OUT_DIR")?);

    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    if target_os != "ios" {
        println!("cargo:rustc-link-lib=dylib=cblite");
    } else {
        println!("cargo:rustc-link-lib=framework=CouchbaseLite");
    }

    Ok(())
}

pub fn copy_lib() -> Result<(), Box<dyn Error>> {
    let lib_path = PathBuf::from(format!(
        "{}/{}/{}/",
        env!("CARGO_MANIFEST_DIR"),
        CBL_LIB_DIR,
        env::var("TARGET").unwrap()
    ));
    let dest_path = PathBuf::from(format!("{}/", env::var("OUT_DIR")?));

    match env::var("CARGO_CFG_TARGET_OS").unwrap().as_str() {
        "android" => {
            fs::copy(
                lib_path.join("libcblite.stripped.so"),
                dest_path.join("libcblite.so"),
            )?;
        }
        "ios" => {
            // Nothing to copy there
        }
        "linux" => {
            fs::copy(
                lib_path.join("libcblite.so.3"),
                dest_path.join("libcblite.so.3"),
            )?;
            // Needed only for build, not required for run
            fs::copy(
                lib_path.join("libcblite.so.3"),
                dest_path.join("libcblite.so"),
            )?;
        }
        "macos" => {
            fs::copy(
                lib_path.join("libcblite.3.dylib"),
                dest_path.join("libcblite.3.dylib"),
            )?;
            // Needed only for build, not required for run
            fs::copy(
                lib_path.join("libcblite.3.dylib"),
                dest_path.join("libcblite.dylib"),
            )?;
        }
        "windows" => {
            fs::copy(lib_path.join("cblite.dll"), dest_path.join("cblite.dll"))?;
            // Needed only for build, not required for run
            fs::copy(lib_path.join("cblite.lib"), dest_path.join("cblite.lib"))?;
        }
        _ => {
            panic!("Unsupported target: {}", env::var("CARGO_CFG_TARGET_OS")?);
        }
    }

    Ok(())
}
