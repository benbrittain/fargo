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

use clap::{App, AppSettings, Arg, SubCommand};
use device::{netaddr, scp_to_device, ssh, start_emulator, stop_emulator};
use sdk::{rust_c_path, rust_linker_path, TargetOptions};
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use update_crates::update_crates;
use utils::strip_binary;

fn run_program_on_target(filename: &str,
                         verbose: bool,
                         target_options: &TargetOptions,
                         launch: bool,
                         params: &[&str])
                         -> Result<()> {
    let netaddr = netaddr(verbose)?;
    if verbose {
        println!("netaddr {}", netaddr);
    }
    let source_path = PathBuf::from(&filename);
    let stripped_source_path = strip_binary(&source_path)?;
    let destination_path = format!("/tmp/{}",
                                   stripped_source_path.file_name()
                                       .unwrap()
                                       .to_string_lossy());
    println!("copying {} to {}",
             source_path.to_string_lossy(),
             destination_path);
    scp_to_device(verbose,
                  &target_options,
                  &netaddr,
                  &stripped_source_path,
                  &destination_path)?;
    let mut command_string = (if launch { "launch " } else { "" }).to_string();
    command_string.push_str(&destination_path);
    for param in params {
        command_string.push(' ');
        command_string.push_str(param);
    }

    if verbose {
        println!("running {}", command_string);
    }
    ssh(verbose, &target_options, &command_string)?;
    Ok(())
}

fn build_tests(verbose: bool,
               release: bool,
               target_options: &TargetOptions,
               test_target: &str)
               -> Result<bool> {
    run_tests(verbose, release, true, target_options, test_target, &vec![])?;
    Ok(true)
}

fn run_tests(verbose: bool,
             release: bool,
             no_run: bool,
             target_options: &TargetOptions,
             test_target: &str,
             params: &[&str])
             -> Result<()> {

    let mut args = vec!["test"];

    if test_target.len() > 0 {
        args.push("--test");
        args.push(test_target);
    }

    if no_run {
        args.push("--no-run");
    }

    for param in params {
        args.push(param);
    }

    run_cargo(verbose, release, false, &args, &target_options)?;
    Ok(())
}

fn build_binary(verbose: bool, release: bool, target_options: &TargetOptions) -> Result<(bool)> {
    run_cargo(verbose, release, false, &vec!["build"], &target_options)
}

fn run_binary(verbose: bool,
              release: bool,
              launch: bool,
              target_options: &TargetOptions)
              -> Result<()> {
    run_cargo(verbose, release, launch, &vec!["run"], &target_options)?;
    return Ok(());
}

fn run_cargo(verbose: bool,
             release: bool,
             launch: bool,
             args: &[&str],
             target_options: &TargetOptions)
             -> Result<(bool)> {
    let mut target_args = vec!["--target", "x86_64-unknown-fuchsia"];

    if release {
        target_args.push("--release");
    }

    if verbose {
        println!("target_args = {:?}", target_args);
    }

    let fargo_path = fs::canonicalize(std::env::current_exe()?)?;

    let fargo_command = if launch {
        format!("{} run-on-target --launch", fargo_path.to_str().unwrap())
    } else {
        format!("{} run-on-target", fargo_path.to_str().unwrap())
    };

    let status = Command::new("cargo").env("RUSTC", rust_c_path()?.to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
             rust_linker_path(&target_options)?.to_str().unwrap())
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_RUNNER", fargo_command)
        .args(args)
        .args(target_args)
        .status()
        .chain_err(|| "Unable to run cargo")?;

    Ok(status.success())
}


fn run() -> Result<()> {
    let matches = App::new("fargo")
        .version("v0.1.0")
        .setting(AppSettings::GlobalVersion)
        .arg(Arg::with_name("verbose")
            .short("v")
            .help("Print verbose output while performing commands"))
        .arg(Arg::with_name("debug-os")
            .long("debug-os")
            .help("Use debug user.bootfs and ssh keys"))
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
        .subcommand(SubCommand::with_name("cargo")
            .about("Run a cargo command for Fuchsia. Use -- to indicate that all \
                         following arguments should be passed to cargo.")
            .arg(Arg::with_name("cargo_params").index(1).multiple(true)))
        .subcommand(SubCommand::with_name("run-on-target")
            .about("Act as a test runner for cargo")
            .arg(Arg::with_name("launch")
                .long("launch")
                .help("Use launch to run binary."))
            .arg(Arg::with_name("run_on_target_params").index(1).multiple(true))
            .setting(AppSettings::Hidden))
        .subcommand(SubCommand::with_name("update-crates")
            .about("Update the FIDL generated crates")
            .arg(Arg::with_name("target")
                .long("target")
                .value_name("target")
                .required(true)
                .help("Target directory for updated crates"))
            .setting(AppSettings::Hidden))
        .get_matches();

    let verbose = matches.is_present("verbose");
    let target_options = TargetOptions::new(!matches.is_present("debug-os"));

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let test_params =
            test_matches.values_of("test_params").map(|x| x.collect()).unwrap_or(vec![]);
        let test_target = test_matches.value_of("test").unwrap_or("");
        return run_tests(verbose,
                         test_matches.is_present("release"),
                         false,
                         &target_options,
                         &test_target,
                         &test_params)
            .chain_err(|| "running tests failed");
    }

    if let Some(build_matches) = matches.subcommand_matches("build") {
        build_binary(verbose,
                     build_matches.is_present("release"),
                     &target_options).chain_err(|| "building binary failed")?;
        return Ok(());
    }

    if let Some(run_matches) = matches.subcommand_matches("run") {
        return run_binary(verbose,
                          run_matches.is_present("release"),
                          run_matches.is_present("launch"),
                          &target_options)
            .chain_err(|| "running binary failed");
    }

    if let Some(build_test_matches) = matches.subcommand_matches("build-tests") {
        let test_target = build_test_matches.value_of("test").unwrap_or("");
        build_tests(verbose,
                    build_test_matches.is_present("release"),
                    &target_options,
                    &test_target).chain_err(|| "building tests failed")?;
        return Ok(());
    }

    if let Some(start_matches) = matches.subcommand_matches("start") {
        return start_emulator(start_matches.is_present("graphics"), &target_options)
            .chain_err(|| "starting emulator failed");
    }

    if let Some(_) = matches.subcommand_matches("stop") {
        return stop_emulator().chain_err(|| "stopping emulator failed");
    }

    if let Some(restart_matches) = matches.subcommand_matches("restart") {
        stop_emulator().chain_err(|| "in restart, stopping emulator failed")?;

        return start_emulator(restart_matches.is_present("graphics"), &target_options)
            .chain_err(|| "in restart, starting emulator failed");
    }

    if let Some(_) = matches.subcommand_matches("ssh") {
        return ssh(verbose, &target_options, "").chain_err(|| "ssh failed");
    }

    if let Some(cargo_matches) = matches.subcommand_matches("cargo") {
        let cargo_params =
            cargo_matches.values_of("cargo_params").map(|x| x.collect()).unwrap_or(vec![]);
        run_cargo(verbose, false, false, &cargo_params,
            &target_options).chain_err(|| "run cargo failed")?;
        return Ok(());
    }

    if let Some(run_on_target_matches) = matches.subcommand_matches("run-on-target") {
        let run_params = run_on_target_matches.values_of("run_on_target_params")
            .map(|x| x.collect())
            .unwrap_or(vec![]);
        let (program, args) = run_params.split_first().unwrap();
        return run_program_on_target(&program, verbose, &target_options, false, &args);
    }

    if let Some(update_matches) = matches.subcommand_matches("update-crates") {
        let update_target = update_matches.value_of("target").unwrap();
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
