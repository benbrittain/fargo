// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

extern crate clap;
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

use cargo_interop::Artifact;
use clap::{App, Arg, SubCommand};
use device::{netaddr, scp_to_device, ssh, start_emulator, stop_emulator};
use sdk::{rust_c_path, rust_linker_path};
use std::path::PathBuf;
use std::process::Command;
use std::str;
use update_crates::update_crates;
use utils::strip_binary;

fn build_tests(verbose: bool, release: bool, test_target: &String) -> bool {
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
    let build_command = Command::new("cargo")
        .env("RUSTC", rust_c_path().to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path().to_str().unwrap())
        .args(args)
        .status()
        .expect("cargo command failed to start");

    build_command.success()
}

fn run_tests(verbose: bool, release: bool, test_target: &String, params: &Vec<String>) {
    if !build_tests(verbose, release, test_target) {
        return;
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

    let output = Command::new("cargo")
        .env("RUSTC", rust_c_path().to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path().to_str().unwrap())
        .args(args)
        .output()
        .expect("cargo command failed to start");

    if output.status.success() {
        let netaddr_result = netaddr();
        match netaddr_result {
            Ok(netaddr) => {
                let artifacts = str::from_utf8(&output.stdout).unwrap().trim().split('\n');
                for artifact_line in artifacts {
                    if verbose {
                        println!("# {}", artifact_line);
                    }
                    let artifact: Artifact = serde_json::from_str(&artifact_line).unwrap();
                    if verbose {
                        println!("# {:?}", artifact);
                    }
                    if artifact.profile.test {
                        for filename in artifact.filenames {
                            let source_path = PathBuf::from(&filename);
                            let stripped_source_path = strip_binary(&source_path);
                            let destination_path = format!("/tmp/{}",
                                                           stripped_source_path.file_name()
                                                               .unwrap()
                                                               .to_string_lossy());
                            println!("copying {} to {}",
                                     source_path.to_string_lossy(),
                                     destination_path);
                            let scp_result =
                                scp_to_device(&netaddr, &stripped_source_path, &destination_path);
                            match scp_result {
                                Ok(_) => {
                                    let command_string = if params.len() > 0 {
                                        let param_string = params.join(" ");
                                        destination_path + " " + &param_string
                                    } else {
                                        destination_path
                                    };
                                    if verbose {
                                        println!("running {}", command_string);
                                    }
                                    ssh(&command_string);
                                }
                                Err(scp_err) => {
                                    println!("scp failed with: {}", scp_err);

                                }
                            }
                        }
                    }
                }
            }

            Err(netaddr_err) => {
                println!("{}", netaddr_err);
            }
        }
    } else {
        println!("cargo test command failed");
    }
}

fn build_binary(verbose: bool, release: bool) -> bool {
    if verbose {
        println!("# build binary phase 1");
    }
    let mut args = vec!["build", "--target", "x86_64-unknown-fuchsia"];
    if release {
        args.push("--release");
    }
    let build_command = Command::new("cargo")
        .env("RUSTC", rust_c_path().to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path().to_str().unwrap())
        .args(args)
        .status()
        .expect("cargo command failed to start");

    build_command.success()
}

fn run_binary(verbose: bool, release: bool) {
    if !build_binary(verbose, release) {
        return;
    }
    let mut args = vec!["build", "--target", "x86_64-unknown-fuchsia"];
    if release {
        args.push("--release");
    }
    let output = Command::new("cargo")
        .env("RUSTC", rust_c_path().to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path().to_str().unwrap())
        .args(args)
        .arg("--message-format")
        .arg("JSON")
        .output()
        .expect("cargo command failed to start");
    if output.status.success() {
        let netaddr_result = netaddr();
        match netaddr_result {
            Ok(netaddr) => {
                let artifacts = str::from_utf8(&output.stdout).unwrap().trim().split('\n');
                for artifact_line in artifacts {
                    if verbose {
                        println!("# {}", artifact_line);
                    }
                    let artifact: Artifact = serde_json::from_str(&artifact_line).unwrap();
                    if verbose {
                        println!("# {:?}", artifact);
                    }
                    if artifact.target.kind.contains(&"bin".to_string()) {
                        for filename in artifact.filenames {
                            let source_path = PathBuf::from(&filename);
                            let stripped_source_path = strip_binary(&source_path);
                            let destination_path = format!("/tmp/{}",
                                                           stripped_source_path.file_name()
                                                               .unwrap()
                                                               .to_string_lossy());
                            println!("copying {} to {}",
                                     stripped_source_path.to_string_lossy(),
                                     destination_path);
                            let scp_result =
                                scp_to_device(&netaddr, &stripped_source_path, &destination_path);
                            match scp_result {
                                Ok(_) => {
                                    println!("running {}", destination_path);
                                    ssh(&destination_path);
                                }
                                Err(scp_err) => {
                                    println!("scp failed with: {}", scp_err);

                                }
                            }
                        }
                    }
                }
            }
            Err(netaddr_err) => {
                println!("{}", netaddr_err);
            }
        }
    }

}

fn main() {
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
                .help("Build release")))
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
        run_tests(verbose,
                  test_matches.is_present("release"),
                  &test_target,
                  &test_params);
    } else if let Some(build_matches) = matches.subcommand_matches("build") {
        build_binary(verbose, build_matches.is_present("release"));
    } else if let Some(run_matches) = matches.subcommand_matches("run") {
        run_binary(verbose, run_matches.is_present("release"));
    } else if let Some(build_test_matches) = matches.subcommand_matches("build-tests") {
        let test_target = build_test_matches.value_of("test").unwrap_or("").to_string();
        build_tests(verbose,
                    build_test_matches.is_present("release"),
                    &test_target);
    } else if let Some(start_matches) = matches.subcommand_matches("start") {
        start_emulator(start_matches.is_present("graphics"));
    } else if let Some(_) = matches.subcommand_matches("stop") {
        stop_emulator();
    } else if let Some(restart_matches) = matches.subcommand_matches("restart") {
        stop_emulator();
        start_emulator(restart_matches.is_present("graphics"));
    } else if let Some(_) = matches.subcommand_matches("ssh") {
        ssh("");
    } else if let Some(update_matches) = matches.subcommand_matches("update-crates") {
        let update_target = update_matches.value_of("target").unwrap().to_string();
        update_crates(&update_target);
    }
}
