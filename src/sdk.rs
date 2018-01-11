// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::path::PathBuf;
use utils::is_mac;

error_chain!{}

/// The `TargetOptions` struct bundles together a number of parameters specific to
/// the Fuchsia target that need to be passed through various internal functions. For
/// the moment there is no way to set anything but the `release_os` field, but this
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
    /// let target_options = TargetOptions::new(true, Some("ivy-donut-grew-stoop"));
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

pub fn fuchsia_root(options: &TargetOptions) -> Result<PathBuf> {
    let fuchsia_root_value = if let Ok(fuchsia_root_value) = env::var("FUCHSIA_ROOT") {
        fuchsia_root_value
    } else {
        let mut path = env::current_dir().unwrap();
        loop {
            if possible_target_out_dir(&path, options).is_ok() {
                return Ok(path);
            }
            path = if let Some(path) = path.parent() {
                path.to_path_buf()
            } else {
                bail!(
                    "FUCHSIA_ROOT not set and current directory is not in a Fuchsia tree with a \
                    release-x86-64 build. You must set the environmental variable FUCHSIA_ROOT to \
                    point to a Fuchsia tree with a release-x86-64 build."
                )
            }
        }
    };

    Ok(PathBuf::from(fuchsia_root_value))
}

pub fn possible_target_out_dir(fuchsia_root: &PathBuf, options: &TargetOptions) -> Result<PathBuf> {
    let out_dir_name_prefix = if options.release_os { "release" } else { "debug" };
    let out_dir_name = format!("{}-{}", out_dir_name_prefix, options.target_cpu);
    let target_out_dir = fuchsia_root.join("out").join(out_dir_name);
    if !target_out_dir.exists() {
        bail!("no target out directory found at  {:?}", target_out_dir);
    }
    Ok(target_out_dir)
}

pub fn target_out_dir(options: &TargetOptions) -> Result<PathBuf> {
    let fuchsia_root = fuchsia_root(options)?;
    possible_target_out_dir(&fuchsia_root, options)
}

pub fn target_gen_dir(options: &TargetOptions) -> Result<PathBuf> {
    let target_out_dir = target_out_dir(options)?;
    Ok(target_out_dir.join("gen"))
}

pub fn strip_tool_path(target_options: &TargetOptions) -> Result<PathBuf> {
    Ok(toolchain_path(target_options)?.join("bin/strip"))
}

pub fn sysroot_path(options: &TargetOptions) -> Result<PathBuf> {
    let zircon_name =
        if options.target_cpu == "x86-64" { "build-user-x86-64" } else { "build-user-arm64" };
    Ok(fuchsia_root(&options)?.join("out").join("build-zircon").join(zircon_name).join("sysroot"))
}

pub fn toolchain_path(target_options: &TargetOptions) -> Result<PathBuf> {
    let platform_name = if is_mac() { "mac-x64" } else { "linux-x64" };
    Ok(fuchsia_root(target_options)?.join("buildtools").join(platform_name).join("clang"))
}

pub fn clang_linker_path(target_options: &TargetOptions) -> Result<PathBuf> {
    Ok(toolchain_path(target_options)?.join("bin").join("clang"))
}
