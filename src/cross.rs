// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use sdk::{TargetOptions, sysroot_path, toolchain_path};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

error_chain!{

    links {
        SDK(::sdk::Error, ::sdk::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
    }
}

pub fn cross_root(target_options: &TargetOptions) -> Result<PathBuf> {
    let home_value = env::var("HOME").chain_err(|| "HOME environmental variable not set")?;

    Ok(PathBuf::from(home_value).join(".fargo").join("native_deps").join(target_options.target_cpu))
}

pub fn pkg_config_path(target_options: &TargetOptions) -> Result<PathBuf> {
    Ok(cross_root(target_options)?.join("lib").join("pkgconfig"))
}

pub fn run_pkg_config(
    verbose: bool,
    args: &[&str],
    target_options: &TargetOptions,
) -> Result<(i32)> {

    let mut cmd = Command::new("pkg-config");

    cmd.args(args)
        .env("PKG_CONFIG_PATH", "")
        .env("PKG_CONFIG_LIBDIR", pkg_config_path(target_options)?)
        .env("PKG_CONFIG_ALL_STATIC", "1");

    if verbose {
        println!("pkg-config: {:?}", cmd);
    }

    cmd.status().chain_err(|| "Unable to run pkg-config").map(|s| match s.code() {
        Some(code) => code,
        None => 1,
    })
}

pub fn run_configure(
    verbose: bool,
    use_host: bool,
    args: &[&str],
    target_options: &TargetOptions,
) -> Result<(bool)> {

    let cwd = fs::canonicalize(env::current_dir()?).chain_err(
        || "run_configure: canonicalize working directory",
    )?;

    let cross_root = cross_root(target_options)?;
    let cross_root_str = cross_root.to_str().unwrap();
    let cross_lib = cross_root.join("lib");
    let cross_lib_str = cross_lib.to_str().unwrap();

    let sysroot_path = sysroot_path(target_options)?;

    if verbose {
        println!("sysroot_path: {:?}", sysroot_path);
    }

    let toolchain_path = toolchain_path(target_options)?;

    if verbose {
        println!("toolchain_path: {:?}", toolchain_path);
    }

    let toolchain_bin_path = toolchain_path.join("bin");

    let common_c_flags = format!(
        "--sysroot={} --target={}-fuchsia -fPIC -I{}",
        sysroot_path.to_str().unwrap(),
        target_options.target_triple(),
        cross_root.join("include").to_str().unwrap()
    );

    let prev_flags = env::var("LDFLAGS").unwrap_or_default();
    let ld_flags = format!("{} {} -L{}", prev_flags, common_c_flags, cross_lib_str);

    if verbose {
        println!("CFLAGS: {}", env::var("CFLAGS").unwrap_or_default());
        println!("LDFLAGS: {}", ld_flags);
    }

    let prefix = format!("--prefix={}", cross_root_str);

    let mut configure_args = vec![];

    if use_host {
        let host =
            if target_options.is_x86() { "x86_64-fuchsia-elf" } else { "aarch64-fuchsia-elf" };
        configure_args.push(host);
    }

    configure_args.push(&prefix);

    let mut cmd = Command::new(cwd.join("configure"));

    cmd.args(&configure_args)
        .args(args)
        .env("CC", toolchain_bin_path.join("clang"))
        .env("CXX", toolchain_bin_path.join("clang++"))
        .env("RANLIB", toolchain_bin_path.join("llvm-ranlib"))
        .env("LD", toolchain_bin_path.join("llvm-lld"))
        .env("AR", toolchain_bin_path.join("llvm-ar"))
        .env("CFLAGS", &common_c_flags)
        .env("CXXFLAGS", &common_c_flags)
        .env("CPPFLAGS", &common_c_flags)
        .env("LDFLAGS", ld_flags)
        .env("PKG_CONFIG_PATH", "")
        .env("PKG_CONFIG_LIBDIR", pkg_config_path(target_options)?)
        .env("PKG_CONFIG_ALL_STATIC", "1");

    if verbose {
        println!("configure: {:?}", cmd);
    }

    cmd.status().chain_err(|| "Unable to run configure").map(|s| s.success())
}
