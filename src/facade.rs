use failure::{Error, ResultExt};
use sdk::{TargetOptions, fuchsia_root};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

fn crate_name_from_path(path: &str) -> Result<String, Error> {
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
    Ok(parts.join("_"))
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

fn create_lib_rs_contents(crate_name: &str) -> String {
    format!(
        r##"
#![deny(warnings)]
#![deny(missing_debug_implementations)]
#![allow(unused_imports)]
#![allow(unused_extern_crates)]

include!(concat!(env!("FUCHSIA_GEN_ROOT"), "{}"));
"##,
        crate_name
    )
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

fn create_facade(path_to_interface: &str, options: &TargetOptions) -> Result<(), Error> {
    let crate_name = crate_name_from_path(path_to_interface)?;
    println!("crate_name {}", crate_name);
    let crate_path =
        fuchsia_root(options)?.join("garnet/public/rust/fidl_crates").join(&crate_name);
    ensure_directory_at_path(&crate_path, "facade crate directory")?;
    let src_path = crate_path.join("src");
    ensure_directory_at_path(&src_path, "facade crate src directory")?;
    let lib_rs_path = src_path.join("lib.rs");
    let mut file = File::create(lib_rs_path).context("can't create or truncate lib.rs file")?;
    let contents = create_lib_rs_contents(&crate_name);
    file.write_all(&contents.as_bytes())?;
    Ok(())
}

pub fn create_facades(interfaces: &Vec<&str>, options: &TargetOptions) -> Result<(), Error> {
    for interface in interfaces {
        let interface = fs::canonicalize(interface)?;
        let fuchsia_root = fuchsia_root(options)?;
        let interface_relative = interface.strip_prefix(&fuchsia_root)?;
        println!("create facade for {:?}", interface_relative.to_str());
        create_facade(interface_relative.to_str().unwrap(), options)?;
    }
    Ok(())
}
