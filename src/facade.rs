use failure::{Error, ResultExt};
use sdk::{TargetOptions, fuchsia_root, fx_path};
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;
use toml;
use toml::Value as Toml;

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    package: Option<Package>,
    dependencies: Option<Toml>,
    workspace: Option<Workspace>,
    patch: Option<PatchTable>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Package {
    name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Workspace {
    members: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PatchTable {
    #[serde(rename = "crates-io")]
    crates_io: Option<BTreeMap<String, Patch>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Patch {
    path: String,
}

#[derive(Debug)]
pub struct FacadeTarget<'a> {
    pub gn_path: &'a str,
    pub fs_path: &'a str,
    pub label: &'a str,
}

impl<'a> FacadeTarget<'a> {
    pub fn parse(path_to_interface: &str) -> Result<FacadeTarget, Error> {
        let label_parts: Vec<&str> = path_to_interface.split(":").collect();
        let (path_without_label, label) = match label_parts.len() {
            1 => (path_to_interface, "fidl"),
            2 => (label_parts[0], label_parts[1]),
            _ => bail!("malformed interface path"),
        };

        let interface_partial_path = if path_without_label.starts_with("//") {
            path_without_label.split_at(2).1
        } else {
            path_without_label
        };

        Ok(FacadeTarget {
            gn_path: path_without_label,
            fs_path: interface_partial_path,
            label: label,
        })
    }

    pub fn crate_name(&self) -> String {
        let mut parts: Vec<&str> = self.fs_path.split("/").filter(|s| s.len() > 0).collect();
        match parts.last() {
            Some(&s) => {
                if s != self.label {
                    parts.push(self.label);
                }
            }
            None => (),
        }
        parts.join("_")
    }
}

#[cfg(test)]
mod tests {
    use facade::FacadeTarget;

    #[test]
    fn test_crate_name_from_path() {
        let facade_target = FacadeTarget::parse("//garnet/public/lib/app/fidl:fidl").unwrap();
        assert_eq!(facade_target.crate_name(), "garnet_public_lib_app_fidl");
        let facade_target = FacadeTarget::parse("//foo/bar/bar:fidl").unwrap();
        assert_eq!(facade_target.crate_name(), "foo_bar_bar_fidl");
        let facade_target = FacadeTarget::parse("//garnet/public/lib/app/fidl:service_provider")
            .unwrap();
        assert_eq!(facade_target.crate_name(), "garnet_public_lib_app_fidl_service_provider");
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

fn format_gn_file(path: &Path, target_options: &TargetOptions) -> Result<(), Error> {
    let fuchsia_root = fuchsia_root(target_options)?;
    let fx_path = fx_path(target_options)?;
    Command::new(fx_path)
        .current_dir(&fuchsia_root)
        .arg("gn")
        .arg("format")
        .arg(path)
        .status()
        .context("failed to run fx gn format")?;
    Ok(())
}

pub fn create_facade(path_to_interface: &str, options: &TargetOptions) -> Result<(), Error> {
    let facade_target = FacadeTarget::parse(path_to_interface)?;
    let fuchsia_root = fuchsia_root(options)?;

    let interface_full_path = fuchsia_root.join(facade_target.fs_path);
    let interface_relative_path = interface_full_path.strip_prefix(&fuchsia_root)?;
    let interface_relative = interface_relative_path.to_str().unwrap();
    let crate_name = facade_target.crate_name();
    let crate_path = fuchsia_root.join("garnet/public/rust/fidl_crates").join(&crate_name);
    let module_name = facade_target.label;

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
    if cargo_toml_path.exists() {
        println!("Warning: Not updating existing Cargo.toml file.")
    } else {
        let mut file =
            File::create(cargo_toml_path).context("can't create or truncate Cargo.toml file")?;
        let contents = create_cargo_toml_contents(&crate_name);
        file.write_all(&contents.as_bytes())?;

    }

    // Create BUILD.gn
    let build_gn_path = crate_path.join("BUILD.gn");
    if build_gn_path.exists() {
        println!("Warning: Not updating existing BUILD.gn file.")
    } else {
        {
            let mut file =
                File::create(&build_gn_path).context("can't create or truncate BUILD.gn file")?;
            let contents = create_build_gn_contents(&crate_name, interface_relative, &module_name);
            file.write_all(&contents.as_bytes())?;
        }
        format_gn_file(&build_gn_path, options)?;
    }

    let garnet_root = fuchsia_root.join("garnet");
    let workspace_path = garnet_root.join("Cargo.toml");
    let mut workspace_file = File::open(&workspace_path)?;
    let mut workspace_contents_str = String::new();
    workspace_file.read_to_string(&mut workspace_contents_str)?;
    let mut decoded: Manifest = toml::from_str(&workspace_contents_str)?;
    {
        let garnet_path = fuchsia_root.join("garnet");
        let relative_crate_path = crate_path.strip_prefix(&garnet_path)?;
        let ref mut workspace = decoded.workspace.as_mut().unwrap();
        let ref mut members = workspace.members.as_mut().unwrap();
        let relative_crate_path_string = String::from(relative_crate_path.to_string_lossy());
        members.push(relative_crate_path_string.clone());
        members.sort();
        members.dedup();
        let patch = Patch { path: relative_crate_path_string};
        let patch_section = decoded.patch.as_mut().unwrap();
        let crates_io = patch_section.crates_io.as_mut().unwrap();
        crates_io.insert(String::from(crate_name), patch);
    }
    let encoded = toml::to_string_pretty(&decoded)?;
    let mut workspace_file = File::create(workspace_path)?;
    workspace_file.write_all(&encoded.as_bytes())?;
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

include!(concat!(env!("FUCHSIA_GEN_ROOT"), "/{}"));
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
