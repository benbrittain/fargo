use sdk::fuchsia_root;
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use toml;

static LICENSE_RS_FILE_HEADER: &'static str =
    r#"// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

"#;

static LICENSE_TOML_FILE_HEADER: &'static str =
    r#"# Copyright 2017 The Fuchsia Authors. All rights reserved.
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

"#;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Dependency {
    path: Option<String>,
    git: Option<String>,
    version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Package {
    name: String,
    version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Library {
    path: String,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Cargo {
    package: Package,
    lib: Option<Library>,
    dependencies: Option<HashMap<String, Dependency>>,
}

impl Cargo {
    fn rewrite(&self) -> Cargo {
        let new_deps = if self.dependencies.is_some() {
            let mut new_deps_map: HashMap<String, Dependency> = HashMap::new();
            let old_deps = self.dependencies.clone().unwrap().clone();
            for (k, mut v) in old_deps {
                v.path = None;
                if k == "magenta" || k == "mxruntime" {
                    v.version = Some("0.1.0".to_string());
                } else {
                    v.git = Some("https://fuchsia.googlesource.com/fuchsia-crates".to_string());
                }
                new_deps_map.insert(k, v);
            }
            Some(new_deps_map)
        } else {
            None
        };
        Cargo {
            package: self.package.clone(),
            lib: self.lib.clone(),
            dependencies: new_deps,
        }
    }
}

fn look_for_crates(dir: &Path, root: &Path, target: &Path) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                look_for_crates(&path, root, target)?;
            } else {
                if let Some(file_name) = path.file_name() {
                    let partial_parent = path.parent().unwrap().strip_prefix(root).unwrap();
                    if file_name.to_str() == Some("Cargo.toml") {
                        let mut input = String::new();
                        File::open(&path)
                            .and_then(|mut f| f.read_to_string(&mut input))
                            .unwrap();
                        let decoded: Cargo = toml::from_str(&input).unwrap();
                        if decoded.lib.is_some() {
                            let rewritten = decoded.rewrite();
                            let target_parent = target.join(partial_parent);
                            fs::create_dir_all(&target_parent).unwrap();
                            let toml2 = toml::to_string(&rewritten).unwrap();
                            let target_cargo = target_parent.join("Cargo.toml");
                            let mut file = File::create(target_cargo)?;
                            file.write_all(&LICENSE_TOML_FILE_HEADER.as_bytes())?;
                            file.write_all(toml2.into_bytes().as_slice())?;
                        }
                    } else {
                        if let Some(extension) = path.extension() {
                            if extension.to_str() == Some("rs") {
                                let target_parent = target.join(partial_parent);
                                fs::create_dir_all(&target_parent).unwrap();
                                let mut input = String::new();
                                File::open(&path)
                                    .and_then(|mut f| f.read_to_string(&mut input))
                                    .unwrap();
                                let target_rust_file = target_parent.join(file_name);
                                let mut file = File::create(target_rust_file)?;
                                file.write_all(&LICENSE_RS_FILE_HEADER.as_bytes())?;
                                file.write_all(input.into_bytes().as_slice())?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn update_crates(target: &String) {
    let gen_root = fuchsia_root().join("out/debug-x86-64/gen");
    let crate_sources = vec!["application", "apps/mozart", "apps/ledger", "apps/modular"];
    for one_source in crate_sources {
        let one_source_path = gen_root.join(one_source);
        look_for_crates(&one_source_path, &gen_root, &PathBuf::from(target)).unwrap();
    }
}
