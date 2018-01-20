use failure::Error;

fn crate_name_from_path(path: &str) -> Result<String, Error> {
    if path.starts_with("/") {
        bail!(format!("illegale FIDL path {}", path));
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


fn create_facade(path_to_interface: &str) -> Result<(), Error> {
    Ok(())
}

pub fn create_facades(interfaces: &Vec<&str>) -> Result<(), Error> {
    for interface in interfaces {
        create_facade(interface)?;
    }
    Ok(())
}
