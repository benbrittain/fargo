// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! While fargo is mainly intended to be a command line tool, this library
//! exposes one function, `run_cargo`, that could be integrated directly into
//! Rust programs that want to cross compile cargo crates on Fuchsia.

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
mod cross;
mod sdk;
mod utils;
mod update_crates;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{
        links {
            Device(::device::Error, ::device::ErrorKind);
            Cross(::cross::Error, ::cross::ErrorKind);
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
use device::{netaddr, netls, scp_to_device, ssh, start_emulator, stop_emulator};
use sdk::{rust_c_path, rust_linker_path};
pub use sdk::TargetOptions;
use cross::{pkg_config_path, run_configure, run_pkg_config};
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use update_crates::update_crates;
use utils::strip_binary;

fn run_program_on_target(
    filename: &str,
    verbose: bool,
    target_options: &TargetOptions,
    launch: bool,
    params: &[&str],
) -> Result<()> {
    let netaddr = netaddr(verbose, &target_options)?;
    if verbose {
        println!("netaddr {}", netaddr);
    }
    let source_path = PathBuf::from(&filename);
    let stripped_source_path = strip_binary(&source_path)?;
    let destination_path = format!(
        "/tmp/{}",
        stripped_source_path.file_name().unwrap().to_string_lossy()
    );
    println!(
        "copying {} to {}",
        source_path.to_string_lossy(),
        destination_path
    );
    scp_to_device(
        verbose,
        &target_options,
        &netaddr,
        &stripped_source_path,
        &destination_path,
    )?;
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

extern crate notify;

use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;

fn autotest(verbose: bool, release: bool, target_options: &TargetOptions) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(1)).chain_err(
        || "autotest: watcher creation failed",
    )?;

    let cwd = std::fs::canonicalize(std::env::current_dir()?).chain_err(
        || "autotest: canonicalize working directory",
    )?;
    let tgt = cwd.join("target");
    let git = cwd.join(".git");

    watcher.watch(&cwd, RecursiveMode::Recursive).chain_err(
        || "autotest: watch failed",
    )?;

    println!("autotest: started");
    loop {
        let event = rx.recv().chain_err(|| "autotest: watch error")?;
        match event {
            notify::DebouncedEvent::Create(path) |
            notify::DebouncedEvent::Write(path) |
            notify::DebouncedEvent::Chmod(path) |
            notify::DebouncedEvent::Remove(path) |
            notify::DebouncedEvent::Rename(path, _) => {
                // TODO(raggi): provide a fuller ignore flag/pattern match solution here.
                if !path.starts_with(&tgt) && !path.starts_with(&git) {
                    println!("autotest: running tests because {:?}", path);
                    run_tests(verbose, release, false, target_options, "", &vec![]).ok();
                }
            }
            _ => {}
        }
    }
}

fn build_tests(
    verbose: bool,
    release: bool,
    target_options: &TargetOptions,
    test_target: &str,
) -> Result<bool> {
    run_tests(verbose, release, true, target_options, test_target, &vec![])?;
    Ok(true)
}

fn run_tests(
    verbose: bool,
    release: bool,
    no_run: bool,
    target_options: &TargetOptions,
    test_target: &str,
    params: &[&str],
) -> Result<()> {

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

    run_cargo(verbose, release, false, &args, &target_options, None)?;
    Ok(())
}

fn build_binary(
    verbose: bool,
    release: bool,
    target_options: &TargetOptions,
    params: &[&str],
) -> Result<()> {
    let mut args = vec!["build"];
    for param in params {
        args.push(param);
    }

    run_cargo(verbose, release, false, &args, &target_options, None)
}

fn run_binary(
    verbose: bool,
    release: bool,
    launch: bool,
    target_options: &TargetOptions,
    params: &[&str],
) -> Result<()> {

    let mut args = vec!["run"];
    for param in params {
        args.push(param);
    }

    run_cargo(verbose, release, launch, &args, &target_options, None)?;
    return Ok(());
}

/// Runs the cargo tool configured to target Fuchsia. When used as a library,
/// the runner options must contain the path to fargo or some other program
/// that implements the `run-on-target` subcommand in a way compatible with
/// fargo.
///
/// # Examples
///
/// ```
/// use fargo::{run_cargo, TargetOptions};
///
/// let target_options = TargetOptions::new(true, None);
/// run_cargo(false, true, false, &["--help"], &target_options, None);
///
/// ```
pub fn run_cargo(
    verbose: bool,
    release: bool,
    launch: bool,
    args: &[&str],
    target_options: &TargetOptions,
    runner: Option<PathBuf>,
) -> Result<()> {
    let mut target_args = vec!["--target", "x86_64-unknown-fuchsia"];

    if release {
        target_args.push("--release");
    }

    if verbose {
        println!("target_args = {:?}", target_args);
    }

    let fargo_path = if runner.is_some() {
        runner.unwrap()
    } else {
        fs::canonicalize(std::env::current_exe()?)?
    };

    let mut runner_args = vec![
        fargo_path.to_str().ok_or_else(
            || "unable to convert path to utf8 encoding"
        )?,
    ];

    if verbose {
        runner_args.push("-v");
    }

    if let Some(device_name) = target_options.device_name {
        runner_args.push("--device-name");
        runner_args.push(device_name);
    }

    runner_args.push("run-on-target");

    if launch {
        runner_args.push("--launch");
    }

    let fargo_command = runner_args.join(" ");

    if verbose {
        println!("fargo_command: {:?}", fargo_command);
    }

    let pkg_path = pkg_config_path(&target_options)?;
    let mut cmd = Command::new("cargo");

    cmd.env("RUSTC", rust_c_path()?.to_str().unwrap())
        .env(
            "CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_LINKER",
            rust_linker_path(&target_options)?.to_str().unwrap(),
        )
        .env("CARGO_TARGET_X86_64_UNKNOWN_FUCHSIA_RUNNER", fargo_command)
        .env("PKG_CONFIG_ALL_STATIC", "1")
        .env("PKG_CONFIG_ALLOW_CROSS", "1")
        .env("PKG_CONFIG_PATH", "")
        .env("PKG_CONFIG_LIBDIR", pkg_path)
        .args(args)
        .args(target_args);

    if verbose {
        println!("cargo cmd: {:?}", cmd);
    }

    let cargo_status = cmd.status()?;
    if !cargo_status.success() {
        bail!(
            "cargo exited with status {:?}",
            cargo_status,
        );
    }

    Ok(())
}


#[doc(hidden)]
pub fn run() -> Result<()> {
    let matches = App::new("fargo")
        .version("v0.1.0")
        .setting(AppSettings::GlobalVersion)
        .about("Fargo is a prototype Fuchsia-specific wrapper around Cargo")
        .arg(Arg::with_name("verbose").long("verbose").short("v").help(
            "Print verbose output while performing commands",
        ))
        .arg(Arg::with_name("debug-os").long("debug-os").help(
            "Use debug user.bootfs and ssh keys",
        ))
        .arg(Arg::with_name("device-name").long("device-name").short("N")
        .value_name("device-name").help(
            "Name of device to target, needed if there are multiple devices visible on the network",
        ))
        .subcommand(
            SubCommand::with_name("autotest")
                .about("Auto build and test in Fuchsia device or emulator")
                .arg(Arg::with_name("release").long("release").help(
                    "Build release",
                )),
        )
        .subcommand(
            SubCommand::with_name("build-tests")
                .about("Build tests for Fuchsia device or emulator")
                .arg(
                    Arg::with_name("test")
                        .long("test")
                        .value_name("test")
                        .help("Test only the specified test target"),
                )
                .arg(Arg::with_name("release").long("release").help(
                    "Build release",
                )),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("Run unit tests on Fuchsia device or emulator")
                .arg(Arg::with_name("release").long("release").help(
                    "Build release",
                ))
                .arg(
                    Arg::with_name("test")
                        .long("test")
                        .value_name("test")
                        .help("Test only the specified test target"),
                )
                .arg(Arg::with_name("test_params").index(1).multiple(true)),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Build binary targeting Fuchsia device or emulator")
                .arg(Arg::with_name("release").long("release").help(
                    "Build release",
                ))
                .arg(
                    Arg::with_name("example")
                        .long("example")
                        .takes_value(true)
                        .help("Build a specific example from the examples/ dir."),
                )
                .arg(Arg::with_name("examples").long("examples").help(
                    "Build all examples in the examples/ dir.",
                )),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run binary on Fuchsia device or emulator")
                .arg(Arg::with_name("release").long("release").help(
                    "Build release",
                ))
                .arg(Arg::with_name("launch").long("launch").help(
                    "Use launch to run binary.",
                ))
                .arg(
                    Arg::with_name("example")
                        .long("example")
                        .value_name("example")
                        .help("Run a specific example from the examples/ dir."),
                ),
        )
        .subcommand(
            SubCommand::with_name("list-devices")
                .about("List visible Fuchsia devices")
        )
        .subcommand(
            SubCommand::with_name("start")
                .about("Start a Fuchsia emulator")
                .arg(Arg::with_name("graphics").short("g").help(
                    "Start a simulator with graphics enabled",
                ))
                .arg(Arg::with_name("no_net"))
                .help("Don't set up networking."),
        )
        .subcommand(SubCommand::with_name("stop").about(
            "Stop all Fuchsia emulators",
        ))
        .subcommand(
            SubCommand::with_name("restart")
                .about("Stop all Fuchsia emulators and start a new one")
                .arg(Arg::with_name("graphics").short("g").help(
                    "Start a simulator with graphics enabled",
                ))
                .arg(Arg::with_name("no_net"))
                .help("Don't set up networking."),
        )
        .subcommand(SubCommand::with_name("ssh").about(
            "Open a shell on Fuchsia device or emulator",
        ))
        .subcommand(
            SubCommand::with_name("cargo")
                .about(
                    "Run a cargo command for Fuchsia. Use -- to indicate that all \
                         following arguments should be passed to cargo.",
                )
                .arg(Arg::with_name("cargo_params").index(1).multiple(true)),
        )
        .subcommand(
            SubCommand::with_name("run-on-target")
                .about("Act as a test runner for cargo")
                .arg(Arg::with_name("launch").long("launch").help(
                    "Use launch to run binary.",
                ))
                .arg(Arg::with_name("run_on_target_params").index(1).multiple(
                    true,
                ))
                .setting(AppSettings::Hidden),
        )
        .subcommand(
            SubCommand::with_name("update-crates")
                .about("Update the FIDL generated crates")
                .arg(
                    Arg::with_name("target")
                        .long("target")
                        .value_name("target")
                        .required(true)
                        .help("Target directory for updated crates"),
                )
                .setting(AppSettings::Hidden),
        )
        .subcommand(
            SubCommand::with_name("pkg-config")
                .about("Run pkg-config for the cross compilation environment")
                .arg(Arg::with_name("pkgconfig_param").index(1).multiple(true)),
        )
        .subcommand(
            SubCommand::with_name("configure")
                .about(
                    "Run a configure script for the cross compilation environment",
                )
                .arg(Arg::with_name("configure_param").index(1).multiple(true))
                .arg(Arg::with_name("no-host").long("no-host").help(
                    "Don't pass --host to configure",
                )),
        )
        .get_matches();

    let verbose = matches.is_present("verbose");
    let target_options = TargetOptions::new(
        !matches.is_present("debug-os"),
        matches.value_of("device-name"),
    );

    if verbose {
        println!("target_options = {:?}", target_options);
    }

    if let Some(autotest_matches) = matches.subcommand_matches("autotest") {
        return autotest(
            verbose,
            autotest_matches.is_present("release"),
            &target_options,
        );
    }

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let test_params = test_matches
            .values_of("test_params")
            .map(|x| x.collect())
            .unwrap_or(vec![]);
        let test_target = test_matches.value_of("test").unwrap_or("");
        return run_tests(
            verbose,
            test_matches.is_present("release"),
            false,
            &target_options,
            &test_target,
            &test_params,
        ).chain_err(|| "running tests failed");
    }

    if let Some(build_matches) = matches.subcommand_matches("build") {

        let mut params = vec![];
        if let Some(example) = build_matches.value_of("example") {
            params.push("--example");
            params.push(example);
        }

        if build_matches.is_present("examples") {
            params.push("--examples");
        }

        build_binary(
            verbose,
            build_matches.is_present("release"),
            &target_options,
            &params,
        ).chain_err(|| "building binary failed")?;
        return Ok(());
    }

    if let Some(run_matches) = matches.subcommand_matches("run") {
        let mut params = vec![];
        if let Some(example) = run_matches.value_of("example") {
            params.push("--example");
            params.push(example);
        }

        return run_binary(
            verbose,
            run_matches.is_present("release"),
            run_matches.is_present("launch"),
            &target_options,
            &params,
        ).chain_err(|| "running binary failed");
    }

    if let Some(build_test_matches) = matches.subcommand_matches("build-tests") {
        let test_target = build_test_matches.value_of("test").unwrap_or("");
        build_tests(
            verbose,
            build_test_matches.is_present("release"),
            &target_options,
            &test_target,
        ).chain_err(|| "building tests failed")?;
        return Ok(());
    }

    if let Some(_) = matches.subcommand_matches("list-devices") {
        return netls(verbose).chain_err(|| "netls failed");
    }

    if let Some(start_matches) = matches.subcommand_matches("start") {
        return start_emulator(
            start_matches.is_present("graphics"),
            !start_matches.is_present("no_net"),
            &target_options,
        ).chain_err(|| "starting emulator failed");
    }

    if let Some(_) = matches.subcommand_matches("stop") {
        return stop_emulator().chain_err(|| "stopping emulator failed");
    }

    if let Some(restart_matches) = matches.subcommand_matches("restart") {
        stop_emulator().chain_err(
            || "in restart, stopping emulator failed",
        )?;

        return start_emulator(
            restart_matches.is_present("graphics"),
            !restart_matches.is_present("no_net"),
            &target_options,
        ).chain_err(|| "in restart, starting emulator failed");
    }

    if let Some(_) = matches.subcommand_matches("ssh") {
        return ssh(verbose, &target_options, "").chain_err(|| "ssh failed");
    }

    if let Some(cargo_matches) = matches.subcommand_matches("cargo") {
        let cargo_params = cargo_matches
            .values_of("cargo_params")
            .map(|x| x.collect())
            .unwrap_or(vec![]);
        run_cargo(verbose, false, false, &cargo_params, &target_options, None)
            .chain_err(|| "run cargo failed")?;
        return Ok(());
    }

    if let Some(run_on_target_matches) = matches.subcommand_matches("run-on-target") {
        let run_params = run_on_target_matches
            .values_of("run_on_target_params")
            .map(|x| x.collect())
            .unwrap_or(vec![]);
        let (program, args) = run_params.split_first().unwrap();
        return run_program_on_target(
            &program,
            verbose,
            &target_options,
            run_on_target_matches.is_present("launch"),
            &args,
        );
    }

    if let Some(update_matches) = matches.subcommand_matches("update-crates") {
        let update_target = update_matches.value_of("target").unwrap();
        return update_crates(&update_target).chain_err(|| "update-crates failed");
    }

    if let Some(pkg_matches) = matches.subcommand_matches("pkg-config") {
        let pkg_params = pkg_matches
            .values_of("pkgconfig_param")
            .map(|x| x.collect())
            .unwrap_or(vec![]);
        let exit_code = run_pkg_config(verbose, &pkg_params, &target_options)
            .chain_err(|| "run_pkg_config failed")?;
        if exit_code != 0 {
            ::std::process::exit(exit_code);
        }
        return Ok(());
    }

    if let Some(configure_matches) = matches.subcommand_matches("configure") {
        let configure_params = configure_matches
            .values_of("configure_param")
            .map(|x| x.collect())
            .unwrap_or(vec![]);
        run_configure(
            verbose,
            !configure_matches.is_present("no-host"),
            &configure_params,
            &target_options,
        ).chain_err(|| "run_configure failed")?;
        return Ok(());
    }

    Ok(())
}
