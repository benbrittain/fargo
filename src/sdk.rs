// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;
use std::env;
use utils::is_mac;

pub fn fuchsia_root() -> PathBuf {
    let fuchsia_root_value = match env::var("FUCHSIA_ROOT") {
        Ok(val) => val,
        Err(_) => {
            panic!("You must set the environmental variable FUCHSIA_ROOT to point to a Fuchsia \
                    tree with a debug-x86-64 build including the rust module")
        }
    };
    PathBuf::from(fuchsia_root_value)
}

fn rust_buildtools_path() -> PathBuf {
    let platform_name = if is_mac() {
        "rust-x86_64-apple-darwin"
    } else {
        "rust-x86_64-unknown-linux-gnu"
    };
    fuchsia_root().join("buildtools/rust").join(platform_name)
}

pub fn rust_c_path() -> PathBuf {
    rust_buildtools_path().join("bin/rustc")
}

pub fn rust_linker_path() -> PathBuf {
    fuchsia_root().join("out/debug-x86-64/host_x64/x86_64-unknown-fuchsia-cc")
}

pub fn strip_tool_path() -> PathBuf {
    let platform_name = if is_mac() {
        "clang+llvm-x86_64-darwin"
    } else {
        "clang+llvm-x86_64-linux"
    };
    fuchsia_root().join("buildtools/toolchain").join(platform_name).join("bin/strip")
}
