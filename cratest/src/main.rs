// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#[macro_use]
extern crate clap;
extern crate git2;
extern crate rayon;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;

use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::iter::FromIterator;
use std::process::Command;
use git2::Repository;
use rayon::prelude::*;
use tempdir::TempDir;

// If unspecified, test this many crates
const DEFAULT_NUM: usize = 5;

#[derive(Debug, Deserialize)]
struct CrateInfo {
    id: String,
    name: String,
    downloads: u64,
    max_version: String,
    description: String,
    homepage: Option<String>,
    repository: String,
}

#[derive(Debug, Deserialize)]
struct Crates {
    crates: Vec<CrateInfo>,
}

#[derive(Debug, Eq, PartialEq)]
enum TestResult {
    Success,
    Failure,
    Excluded,
}

#[derive(Debug)]
struct CrateResult {
    id: String,
    res: TestResult,
}

fn main() {
    let matches = clap_app!(cratest =>
                  (version: "1.0")
                  (author: "Tim Kilbourn <tkilbourn@google.com>")
                  (about: "Tests the top crates from crates.io on Fuchsia")
                  (@arg num: -n +takes_value "Number of crates to test")
                  (@arg excludes: -x --exclude ... +takes_value
                        "Exclude crates whose name exactly match")
                  (@arg start: --start "Starts a Fuchsia emulator")
                  (@arg restart: --restart "Stop all Fuchsia emulators and start a new one")
                  (@arg keep: --keep "Keeps the temp dir after exiting")
                  (@arg verbose: -v --verbose "Print verbose output while performing commands")
                 ).get_matches();

    let num = value_t!(matches, "num", usize).unwrap_or(DEFAULT_NUM);
    let verbose = matches.is_present("verbose");
    let restart_emu = matches.is_present("restart");
    let start_emu = matches.is_present("start");
    let keep_tmp = matches.is_present("keep");
    let excludes =
        HashSet::<String>::from_iter(values_t!(matches, "excludes", String).unwrap_or(Vec::new()));

    println!(
        "Running cratest on the top {} crates from crates.io...",
        num
    );

    let crate_uri: String = [
        "https://crates.io/api/v1/crates?page=1&per_page=",
        &format!("{}", num),
        "&sort=downloads",
    ].join("")
        .into();

    if verbose {
        println!("Downloading crates from {}", crate_uri);
        if excludes.len() > 0 {
            println!("Excluding {} crates", excludes.len());
        }
    }
    let mut resp = reqwest::get(&crate_uri).unwrap();
    assert!(resp.status().is_success());

    let mut content = String::new();
    resp.read_to_string(&mut content).unwrap();

    let res: Crates = serde_json::from_str(&content).unwrap();

    if restart_emu {
        Command::new("fargo").arg("restart").status().expect(
            "failed to run fargo restart",
        );
    } else if start_emu {
        Command::new("fargo").arg("start").status().expect(
            "failed to run fargo start",
        );
    }

    let tmpdir = TempDir::new("cratest").unwrap();

    let results: Vec<CrateResult> = res.crates
        .par_iter()
        .map(|ref cr| {
            if excludes.contains(&cr.id) {
                if verbose {
                    println!("Skipping {} (excluded)", &cr.id);
                }
                return CrateResult {
                    id: cr.id.clone(),
                    res: TestResult::Excluded,
                };
            }
            let crdir = tmpdir.path().join(&cr.id);
            fs::create_dir(&crdir).unwrap();
            Repository::clone(&cr.repository, &crdir).unwrap();

            let output = Command::new("fargo")
                .arg("test")
                .current_dir(&crdir)
                .output()
                .expect("failed to execute fargo test");
            println!("crate: {}", &cr.id);
            println!("status: {}", output.status);
            if verbose {
                println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
            }

            CrateResult {
                id: cr.id.clone(),
                res: if output.status.success() {
                    TestResult::Success
                } else {
                    TestResult::Failure
                },
            }
        })
        .collect();

    let (succ, fail, excl) = results.into_iter().fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut s, mut f, mut e), r| {
            match r.res {
                TestResult::Success => s.push(r.id),
                TestResult::Failure => f.push(r.id),
                TestResult::Excluded => e.push(r.id),
            }
            (s, f, e)
        },
    );

    for &(hdr, ref results) in &[("Successes", succ), ("Failures", fail), ("Excluded", excl)] {
        if results.len() > 0 {
            println!("{}({}): {:?}", hdr, results.len(), results);
        }
    }

    if keep_tmp {
        let tmppath = tmpdir.into_path();
        println!("Temp output left at {}", tmppath.to_string_lossy());
    }
}
