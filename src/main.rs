// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![recursion_limit = "1024"]

extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate toml;
extern crate uname;

mod cargo_interop;
mod device;
mod sdk;
mod utils;
mod update_crates;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{
        links {
            Device(::device::Error, ::device::ErrorKind);
            SDK(::sdk::Error, ::sdk::ErrorKind);
            Utils(::utils::Error, ::utils::ErrorKind);
        }

        foreign_links {
            Io(::std::io::Error);
        }
    }
}

use errors::*;

use cargo_interop::Artifact;
use clap::{App, Arg, SubCommand};
use device::{netaddr, scp_to_device, ssh, start_emulator, stop_emulator};
use sdk::{rust_c_path, rust_linker_path};
use std::path::PathBuf;
use std::process::Command;
use std::str;
use update_crates::update_crates;
use utils::strip_binary;

fn run_programs_on_target(programs: &Vec<String>,
                          verbose: bool,
                          launch: bool,
                          params: &[String])
                          -> Result<()> {
    let netaddr = netaddr(verbose)?;
    for filename in programs {
        let source_path = PathBuf::from(&filename);
        let stripped_source_path = strip_binary(&source_path)?;
        let destination_path = format!("/tmp/{}",
                                       stripped_source_path.file_name()
                                           .unwrap()
                                           .to_string_lossy());
        println!("copying {} to {}",
                 source_path.to_string_lossy(),
                 destination_path);
        scp_to_device(verbose, &netaddr, &stripped_source_path, &destination_path)?;
        let mut command_string = (if launch { "launch " } else { "" }).to_string();
        command_string.push_str(&destination_path);
        for param in params {
            command_string.push(' ');
            command_string.push_str(param);
        }

        if verbose {
            println!("running {}", command_string);
        }
        ssh(verbose, &command_string)?
    }
    Ok(())
}

fn programs_from_artifacts<F>(verbose: bool, artifacts_text: &str, filter: F) -> Vec<String>
    where F: Fn(&Artifact) -> bool
{
    let mut programs = vec![];
    let artifacts = artifacts_text.trim().split('\n');
    for artifact_line in artifacts {
        if verbose {
            println!("# {}", artifact_line);
        }
        let artifact: Artifact = serde_json::from_str(&artifact_line).unwrap();
        if verbose {
            println!("# {:?}", artifact);
        }
        if filter(&artifact) {
            for filename in artifact.filenames {
                programs.push(filename);
            }
        }
    }
    programs
}


fn build_tests(verbose: bool, release: bool, test_target: &String) -> Result<bool> {
    if verbose {
        println!("# build tests phase 1");
    }

    let mut args = vec!["test", "--target", "x86_64-unknown-fuchsia", "--no-run"];

    if release {
        args.push("--release");
    }

    if test_target.len() > 0 {
        args.push("--test");
        args.push(test_target.as_str());
    }

    let status = Command::new("cargo").env("RUSTC", rust_c_path()?.to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path()?.to_str().unwrap())
        .args(args)
        .status()
        .chain_err(|| "Unable to run cargo test")?;

    Ok(status.success())
}

fn run_tests(verbose: bool,
             release: bool,
             test_target: &String,
             params: &Vec<String>)
             -> Result<()> {

    if !build_tests(verbose, release, test_target)? {
        return Ok(());
    }

    if verbose {
        println!("# build tests phase 2");
    }
    let mut args = vec!["test",
                        "--target",
                        "x86_64-unknown-fuchsia",
                        "-q",
                        "--no-run",
                        "--message-format",
                        "JSON"];
    if release {
        args.push("--release");
    }

    if test_target.len() > 0 {
        args.push("--test");
        args.push(test_target.as_str());
    }

    let output = Command::new("cargo").env("RUSTC", rust_c_path()?.to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path()?.to_str().unwrap())
        .args(args)
        .output()
        .chain_err(|| "Unable to run cargo test")?;

    let artifacts = str::from_utf8(&output.stdout).unwrap();
    let programs = programs_from_artifacts(verbose, artifacts, |artifact| artifact.profile.test);

    run_programs_on_target(&programs, verbose, false, &params)
}

fn build_binary(verbose: bool, release: bool) -> Result<(bool)> {
    if verbose {
        println!("# build binary phase 1");
    }

    let mut args = vec!["build", "--target", "x86_64-unknown-fuchsia"];
    if release {
        args.push("--release");
    }

    let status = Command::new("cargo").env("RUSTC", rust_c_path()?.to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path()?.to_str().unwrap())
        .args(args)
        .status()
        .chain_err(|| "Unable to run cargo build")?;

    Ok(status.success())
}

fn run_binary(verbose: bool, release: bool, launch: bool) -> Result<()> {

    if !build_binary(verbose, release)? {
        return Ok(());
    }

    let mut args = vec!["build", "--target", "x86_64-unknown-fuchsia"];
    if release {
        args.push("--release");
    }
    let output = Command::new("cargo").env("RUSTC", rust_c_path()?.to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path()?.to_str().unwrap())
        .args(args)
        .arg("--message-format")
        .arg("JSON")
        .output()
        .chain_err(|| "Unable to run cargo build")?;

    let artifacts = str::from_utf8(&output.stdout).unwrap();
    let programs = programs_from_artifacts(verbose, artifacts, |artifact| {
        artifact.target.kind.contains(&"bin".to_string())
    });

    run_programs_on_target(&programs, verbose, launch, &[])
}

fn run() -> Result<()> {
    let matches = App::new("fargo")
        .version("v0.1.0")
        .arg(Arg::with_name("verbose")
            .short("v")
            .help("Print verbose output while performing commands"))
        .subcommand(SubCommand::with_name("build-tests")
            .about("Build for Fuchsia device or emulator")
            .arg(Arg::with_name("test")
                .long("test")
                .value_name("test")
                .help("Test only the specified test target"))
            .arg(Arg::with_name("release")
                .long("release")
                .help("Build release")))
        .subcommand(SubCommand::with_name("test")
            .about("Run unit tests on Fuchsia device or emulator")
            .arg(Arg::with_name("release")
                .long("release")
                .help("Build release"))
            .arg(Arg::with_name("test")
                .long("test")
                .value_name("test")
                .help("Test only the specified test target"))
            .arg(Arg::with_name("test_params").index(1).multiple(true)))
        .subcommand(SubCommand::with_name("build")
            .about("Build binary targeting Fuchsia device or emulator")
            .arg(Arg::with_name("release")
                .long("release")
                .help("Build release")))
        .subcommand(SubCommand::with_name("run")
            .about("Run binary on Fuchsia device or emulator")
            .arg(Arg::with_name("release")
                .long("release")
                .help("Build release"))
            .arg(Arg::with_name("launch")
                .long("launch")
                .help("Use launch to run binary.")))
        .subcommand(SubCommand::with_name("start")
            .about("Start a Fuchsia emulator")
            .arg(Arg::with_name("graphics")
                .short("g")
                .help("Start a simulator with graphics enabled")))
        .subcommand(SubCommand::with_name("stop").about("Stop all Fuchsia emulators"))
        .subcommand(SubCommand::with_name("restart")
            .about("Stop all Fuchsia emulators and start a new one")
            .arg(Arg::with_name("graphics")
                .short("g")
                .help("Start a simulator with graphics enabled")))
        .subcommand(SubCommand::with_name("ssh")
            .about("Open a shell on Fuchsia device or emulator"))
        .subcommand(SubCommand::with_name("update-crates")
            .about("Update the FIDL generated crates")
            .arg(Arg::with_name("target")
                .long("target")
                .value_name("target")
                .required(true)
                .help("Target directory for updated crates")))
        .get_matches();

    let verbose = matches.is_present("verbose");

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let test_params = test_matches.values_of_lossy("test_params").unwrap_or(vec![]);
        let test_target = test_matches.value_of("test").unwrap_or("").to_string();
        return run_tests(verbose,
                         test_matches.is_present("release"),
                         &test_target,
                         &test_params)
            .chain_err(|| "running tests failed");
    }

    if let Some(build_matches) = matches.subcommand_matches("build") {
        build_binary(verbose, build_matches.is_present("release"))
            .chain_err(|| "building binary failed")?;
        return Ok(());
    }

    if let Some(run_matches) = matches.subcommand_matches("run") {
        return run_binary(verbose,
                          run_matches.is_present("release"),
                          run_matches.is_present("launch"))
            .chain_err(|| "running binary failed");
    }

    if let Some(build_test_matches) = matches.subcommand_matches("build-tests") {
        let test_target = build_test_matches.value_of("test").unwrap_or("").to_string();
        build_tests(verbose,
                    build_test_matches.is_present("release"),
                    &test_target).chain_err(|| "building tests failed")?;
        return Ok(());
    }

    if let Some(start_matches) = matches.subcommand_matches("start") {
        return start_emulator(start_matches.is_present("graphics"))
            .chain_err(|| "starting emulator failed");
    }

    if let Some(_) = matches.subcommand_matches("stop") {
        return stop_emulator().chain_err(|| "stopping emulator failed");
    }

    if let Some(restart_matches) = matches.subcommand_matches("restart") {
        stop_emulator().chain_err(|| "in restart, stopping emulator failed")?;

        return start_emulator(restart_matches.is_present("graphics"))
            .chain_err(|| "in restart, starting emulator failed");
    }

    if let Some(_) = matches.subcommand_matches("ssh") {
        return ssh(verbose, "").chain_err(|| "ssh failed");
    }

    if let Some(update_matches) = matches.subcommand_matches("update-crates") {
        let update_target = update_matches.value_of("target").unwrap().to_string();
        return update_crates(&update_target).chain_err(|| "update-crates failed");
    }

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}
