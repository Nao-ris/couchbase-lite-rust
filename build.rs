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
extern crate fs_extra;

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use fs_extra::dir;

static CBL_INCLUDE_DIR: &str = "libcblite-3.0.3/include";
static CBL_LIB_DIR: &str = "libcblite-3.0.3/lib";

fn main() -> Result<(), Box<dyn Error>> {
    generate_bindings()?;
    configure_rustc()?;

    // Bypass copying libraries when the build script is called in a cargo check context.
    if env::var("ONLY_CARGO_CHECK").unwrap_or_default() != *"true" {
        copy_lib().unwrap_or_else(|_| {
            panic!(
                "can't copy cblite libs, is '{}' a supported target?",
                env::var("TARGET").unwrap_or_default()
            )
        });
    }

    Ok(())
}

fn bindgen_for_mac(builder: bindgen::Builder) -> Result<bindgen::Builder, Box<dyn Error>> {
    let target = env::var("TARGET")?;
    if !target.contains("apple") {
        return Ok(builder);
    }

    let sdk = if target.contains("ios") {
        if target.contains("x86") || target.contains("sim") {
            "iphonesimulator"
        } else {
            "iphoneos"
        }
    } else {
        "macosx"
    };

    let sdk = String::from_utf8(
        Command::new("xcrun")
            .args(["--sdk", sdk, "--show-sdk-path"])
            .output()
            .expect("failed to execute process")
            .stdout,
    )?;

    Ok(builder.clang_arg(format!("-isysroot{}", sdk.trim())))
}

#[allow(dead_code)]
enum OperatingSystem {
    MacOs,
    Windows,
    Android,
    IOs,
}

fn is_target(target: OperatingSystem) -> Result<bool, Box<dyn Error>> {
    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    Ok(match target {
        OperatingSystem::Android => target_os.contains("android"),
        OperatingSystem::Windows => target_os.contains("windows"),
        OperatingSystem::IOs => target_os.contains("apple-ios"),
        OperatingSystem::MacOs => target_os.contains("apple-darwin"),
    })
}

fn is_host(host: OperatingSystem) -> Result<bool, Box<dyn Error>> {
    let host_os = env::var("HOST")?;
    Ok(match host {
        OperatingSystem::MacOs => host_os.contains("apple-darwin"),
        OperatingSystem::Windows => host_os.contains("windows"),
        OperatingSystem::Android => host_os.contains("android"),
        OperatingSystem::IOs => host_os.contains("apple-ios"),
    })
}

fn generate_bindings() -> Result<(), Box<dyn Error>> {
    let mut bindings = bindgen_for_mac(bindgen::Builder::default())?
        .header("src/wrapper.h")
        .clang_arg(format!("-I{}", CBL_INCLUDE_DIR));

    // Fix cross-compilation from MacOS to Android targets.
    // The following clang_arg calls prevent bindgen from trying to include
    // MacOS standards headers and returning an error when trying to generate bindings.
    // Basically, we specifiy NDK sysroot and usr/include dirs depending on the target arch.
    //
    // Sample of errors:
    //
    // /Applications/Xcode.app/.../Developer/SDKs/MacOSX10.15.sdk/usr/include/sys/cdefs.h:807:2: error: Unsupported architecture
    // /Applications/Xcode.app/.../Developer/SDKs/MacOSX10.15.sdk/usr/include/machine/_types.h:34:2: error: architecture not supported
    // FTR: https://github.com/rust-lang/rust-bindgen/issues/1780
    if is_host(OperatingSystem::MacOs)? && is_target(OperatingSystem::Android)? {
        let ndk_sysroot = format!(
            "{}/toolchains/llvm/prebuilt/darwin-x86_64/sysroot",
            env::var("NDK_HOME")?,
        );
        let target_triplet =
            if env::var("CARGO_CFG_TARGET_ARCH").expect("Can't read target arch value!") == "arm" {
                "arm-linux-androideabi"
            } else {
                "aarch64-linux-android"
            };
        bindings = bindings
            .clang_arg(format!("--sysroot={}", ndk_sysroot))
            .clang_arg(format!("-I{}/usr/include", ndk_sysroot))
            .clang_arg(format!("-I{}/usr/include/{}", ndk_sysroot, target_triplet))
            .clang_arg(format!("--target={}", target_triplet));
    }

    // Cross compiling from Mac to Windows
    if is_host(OperatingSystem::MacOs)? && is_target(OperatingSystem::Windows)? {
        let homebrew_prefix = Command::new("brew")
            .arg("--prefix")
            .arg("mingw-w64")
            .output()?
            .stdout;
        let homebrew_prefix =
            String::from_utf8_lossy(&homebrew_prefix[..homebrew_prefix.len() - 1]);
        let mingw_path = format!("{homebrew_prefix}/toolchain-x86_64/x86_64-w64-mingw32");
        let mingw_include_path = format!("{mingw_path}/include");
        bindings = bindings
            .clang_arg(format!("-I{}", mingw_include_path))
            .clang_arg(format!("--target={}", "x86_64-pc-windows-gnu"));
    }

    let out_dir = env::var("OUT_DIR")?;
    bindings
        .allowlist_type("CBL.*")
        .allowlist_type("FL.*")
        .allowlist_var("k?CBL.*")
        .allowlist_var("k?FL.*")
        .allowlist_function("CBL.*")
        .allowlist_function("_?FL.*")
        .no_copy("FLSliceResult")
        .size_t_is_usize(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(out_dir).join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Ok(())
}

fn configure_rustc() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:rerun-if-changed={}", CBL_INCLUDE_DIR);
    println!("cargo:rerun-if-changed={}", CBL_LIB_DIR);
    println!("cargo:rustc-link-search={}", env::var("OUT_DIR")?);

    let target_dir = env::var("TARGET")?;
    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    if target_os == "ios" {
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("Can't read target_arch");
        let ios_framework = match target_arch.as_str() {
            "aarch64" => "ios-arm64",
            "x86_64" => "ios-arm64_x86_64-simulator",
            _ => panic!("Unsupported ios target"),
        };

        println!(
            "cargo:rustc-link-search=framework={}/{}/ios/CouchbaseLite.xcframework/{}",
            env!("CARGO_MANIFEST_DIR"),
            CBL_LIB_DIR,
            ios_framework,
        );
        println!("cargo:rustc-link-lib=framework=CouchbaseLite");
    } else if target_os == "macos" {
        println!("cargo:rustc-link-lib=dylib=cblite");
        println!(
            "cargo:rustc-link-search={}/{}/macos",
            env!("CARGO_MANIFEST_DIR"),
            CBL_LIB_DIR
        );
    } else {
        println!("cargo:rustc-link-lib=dylib=cblite");
        println!(
            "cargo:rustc-link-search={}/{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            CBL_LIB_DIR,
            target_dir
        );
    }
    Ok(())
}

pub fn copy_lib() -> Result<(), Box<dyn Error>> {
    let target_os = env::var("CARGO_CFG_TARGET_OS")?;
    let lib_path = PathBuf::from(format!(
        "{}/{}/{}/",
        env!("CARGO_MANIFEST_DIR"),
        CBL_LIB_DIR,
        if target_os == "ios" {
            "ios".to_string()
        } else if target_os == "macos" {
            "macos".to_string()
        } else {
            env::var("TARGET").unwrap()
        }
    ));
    let dest_path = PathBuf::from(format!("{}/", env::var("OUT_DIR")?));

    match target_os.as_str() {
        "android" => {
            fs::copy(
                lib_path.join("libcblite.stripped.so"),
                dest_path.join("libcblite.so"),
            )?;
        }
        "ios" => {
            let mut copy_options = dir::CopyOptions::new();
            copy_options.overwrite = true;

            dir::copy(
                lib_path.join("CouchbaseLite.xcframework"),
                dest_path,
                &copy_options,
            )?;
        }
        "linux" => {
            fs::copy(
                lib_path.join("libcblite.so.3"),
                dest_path.join("libcblite.so.3"),
            )?;
            fs::copy(
                lib_path.join("libicudata.so.66"),
                dest_path.join("libicudata.so.66"),
            )?;
            fs::copy(
                lib_path.join("libicui18n.so.66"),
                dest_path.join("libicui18n.so.66"),
            )?;
            fs::copy(
                lib_path.join("libicuio.so.66"),
                dest_path.join("libicuio.so.66"),
            )?;
            fs::copy(
                lib_path.join("libicutu.so.66"),
                dest_path.join("libicutu.so.66"),
            )?;
            fs::copy(
                lib_path.join("libicuuc.so.66"),
                dest_path.join("libicuuc.so.66"),
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
