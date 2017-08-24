// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;
use std::env;
use utils::is_mac;

error_chain!{}

/// The `TargetOptions` struct bundles together a number of parameters specific to
/// the Fuchsia target that need to be passed through various internal functions. For
/// the moment there is no way to set anything but the release_os field, but this
/// will change when fargo starts supporting ARM targets.
#[derive(Debug)]
pub struct TargetOptions<'a> {
    pub release_os: bool,
    pub target_cpu: &'a str,
    pub target_cpu_linker: &'a str,
    pub device_name: Option<&'a str>,
}

impl<'a> TargetOptions<'a> {
    /// Constructs a new `TargetOptions`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fargo::TargetOptions;
    ///
    /// let target_options = TargetOptions::new(true);
    /// ```

    pub fn new(release_os: bool, device_name: Option<&'a str>) -> TargetOptions {
        TargetOptions {
            release_os: release_os,
            target_cpu: "x86-64",
            target_cpu_linker: "x86_64",
            device_name: device_name,
        }
    }
}

pub fn fuchsia_root() -> Result<PathBuf> {
    let fuchsia_root_value = env::var("FUCHSIA_ROOT").chain_err(|| {
        "FUCHSIA_ROOT not set. You must set the environmental variable FUCHSIA_ROOT to point \
             to a Fuchsia tree with a debug-x86-64 build including the rust module"
    })?;

    Ok(PathBuf::from(fuchsia_root_value))
}

pub fn target_out_dir(options: &TargetOptions) -> Result<PathBuf> {
    let fuchsia_root = fuchsia_root()?;
    let out_dir_name_prefix = if options.release_os {
        "release"
    } else {
        "debug"
    };
    let out_dir_name = format!("{}-{}", out_dir_name_prefix, options.target_cpu);
    let target_out_dir = fuchsia_root.join("out").join(out_dir_name);
    if !target_out_dir.exists() {
        bail!("no target out directory found at  {:?}", target_out_dir);
    }
    Ok(target_out_dir)
}

fn rust_buildtools_path() -> Result<PathBuf> {
    let platform_name = if is_mac() { "mac-x64" } else { "linux-x64" };
    Ok(
        fuchsia_root()?
            .join("buildtools")
            .join(platform_name)
            .join("rust"),
    )
}

pub fn rust_c_path() -> Result<PathBuf> {
    Ok(rust_buildtools_path()?.join("bin/rustc"))
}

pub fn rust_linker_path(options: &TargetOptions) -> Result<PathBuf> {
    let linker_name = format!("{}-unknown-fuchsia-cc", options.target_cpu_linker);
    Ok(target_out_dir(&options)?.join("host_x64").join(linker_name))
}

pub fn strip_tool_path() -> Result<PathBuf> {
    Ok(toolchain_path()?.join("bin/strip"))
}

pub fn sysroot_path(options: &TargetOptions) -> Result<PathBuf> {
    let magenta_name = if options.target_cpu == "x86-64" {
        "build-magenta-pc-x86-64"
    } else {
        "build-magenta-qemu-arm64"
    };
    Ok(
        fuchsia_root()?
            .join("out")
            .join("build-magenta")
            .join(magenta_name)
            .join("sysroot"),
    )
}

pub fn toolchain_path() -> Result<PathBuf> {
    let platform_name = if is_mac() {
        "clang+llvm-x86_64-darwin"
    } else {
        "clang+llvm-x86_64-linux"
    };
    Ok(fuchsia_root()?.join("buildtools").join("toolchain").join(
        platform_name,
    ))
}
