use failure::{Error, ResultExt};
use sdk::{TargetOptions, fuchsia_root};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;


fn crate_name_parts(path: &str) -> Result<Vec<&str>, Error> {
    if path.starts_with("/") {
        bail!(format!("illegal FIDL path {}", path));
    }
    let mut parts: Vec<&str> = path.split("/").collect();
    if parts.len() > 2 {
        let index = parts.len() - 2;
        if parts[index] == parts[index + 1] {
            parts.pop();
        }
    }
    Ok(parts)
}

fn crate_name_from_path(path: &str) -> Result<String, Error> {
    let parts = crate_name_parts(path)?;
    Ok(parts.join("_"))
}

fn module_name_from_path(path: &str) -> Result<String, Error> {
    let parts = crate_name_parts(path)?;
    let module_name = parts.last().unwrap_or(&"");
    Ok(module_name.to_string())
}

#[cfg(test)]
mod tests {
    use facade::crate_name_from_path;

    #[test]
    fn test_crate_name_from_path() {
        let name = crate_name_from_path("garnet/public/lib/app/fidl/fidl").unwrap();
        assert_eq!(name, "garnet_public_lib_app_fidl");
        assert_eq!(crate_name_from_path("foo/bar/bar").unwrap(), "foo_bar");
    }
}

fn ensure_directory_at_path(path: &Path, path_description: &str) -> Result<(), Error> {
    if path.exists() {
        if !path.is_dir() {
            bail!("{:?} already exists but isn't a directory", path);
        }
    } else {
        fs::create_dir_all(&path).context(format!("Can't create {} {:?}", path_description, path))?;
    }
    Ok(())
}

pub fn create_facade(path_to_interface: &str, options: &TargetOptions) -> Result<(), Error> {
    let interface = fs::canonicalize(path_to_interface)?;
    let fuchsia_root = fuchsia_root(options)?;
    let interface_relative_path = interface.strip_prefix(&fuchsia_root)?;
    let interface_relative = interface_relative_path.to_str().unwrap();
    let crate_name = crate_name_from_path(interface_relative)?;
    let crate_path = fuchsia_root.join("garnet/public/rust/fidl_crates").join(&crate_name);

    // Create lib.rs
    ensure_directory_at_path(&crate_path, "facade crate directory")?;
    let src_path = crate_path.join("src");
    ensure_directory_at_path(&src_path, "facade crate src directory")?;
    let lib_rs_path = src_path.join("lib.rs");
    let mut file = File::create(lib_rs_path).context("can't create or truncate lib.rs file")?;
    let contents = create_lib_rs_contents(&format!("{}/{}.rs", interface_relative, crate_name));
    file.write_all(&contents.as_bytes())?;

    // Create Cargo.toml
    let cargo_toml_path = crate_path.join("Cargo.toml");
    let mut file =
        File::create(cargo_toml_path).context("can't create or truncate Cargo.toml file")?;
    let contents = create_cargo_toml_contents(&crate_name);
    file.write_all(&contents.as_bytes())?;

    // Create BUILD.gn
    let build_gn_path = crate_path.join("BUILD.gn");
    let mut file = File::create(build_gn_path).context("can't create or truncate BUILD.gn file")?;
    let contents = create_build_gn_contents(
        &crate_name,
        interface_relative,
        &module_name_from_path(interface_relative)?,
    );
    file.write_all(&contents.as_bytes())?;

    println!("Created or updated facade crate at {:?}.", crate_path);
    Ok(())
}

fn create_lib_rs_contents(path_to_generated_file: &str) -> String {
    format!(
        r##"// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
#![deny(warnings)]
#![deny(missing_debug_implementations)]
#![allow(unused_imports)]
#![allow(unused_extern_crates)]

include!(concat!(env!("FUCHSIA_GEN_ROOT"), "{}"));
"##,
        path_to_generated_file
    )
}

fn create_cargo_toml_contents(crate_name: &str) -> String {
    format!(
        r##"# Copyright 2018 The Fuchsia Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.
[package]
name = "{}"
version = "0.1.0"
license = "BSD-3-Clause"
authors = ["rust-fuchsia@fuchsia.com"]
description = "Generated interface"
repository = "https://fuchsia.googlesource.com/garnet/"

[dependencies]
fidl = "0.1"
fuchsia-zircon = "0.3"
futures = "0.1.15"
tokio-core = "0.1"
tokio-fuchsia = "0.1"
"##,
        crate_name
    )
}

fn create_build_gn_contents(crate_name: &str, interface_path: &str, module_name: &str) -> String {
    format!(
        r##"# Copyright 2017 The Fuchsia Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

import("//build/rust/rust_library.gni")

rust_library("{}") {{
  deps = [
    "//{}:{}_rust",
  ]
  }}
"##,
        crate_name,
        interface_path,
        module_name
    )
}
