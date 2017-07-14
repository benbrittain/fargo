// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str;
use sdk::fuchsia_root;
use utils::is_mac;

pub fn netaddr(verbose: bool) -> Result<String, String> {
    let fuchsia_root = fuchsia_root();
    let netaddr_binary = fuchsia_root.join("out/build-magenta/tools/netaddr");
    let netaddr_result = Command::new(netaddr_binary)
        .arg("--fuchsia")
        .output()
        .expect("Couldn't run netaddr.");
    if netaddr_result.status.success() {
        let result = str::from_utf8(&netaddr_result.stdout).unwrap().trim().to_string();
        if verbose {
            println!("netaddr result = {}", result);
        }
        Ok(result)
    } else {
        Err(format!("netaddr failed with: {}",
                    String::from_utf8_lossy(&netaddr_result.stderr)))
    }
}

pub fn scp_to_device(verbose: bool,
                     netaddr: &String,
                     source_path: &PathBuf,
                     destination_path: &String)
                     -> Result<(), String> {
    let destination_with_address = format!("[{}]:{}", netaddr, destination_path);
    let fuchsia_root = fuchsia_root();
    let ssh_config = fuchsia_root.join("out/debug-x86-64/ssh-keys/ssh_config");
    let ssh_result = Command::new("scp")
        .arg(if verbose { "-v" } else { "-q" })
        .arg("-F")
        .arg(ssh_config)
        .arg(source_path)
        .arg(destination_with_address)
        .status()
        .expect("Unable to run scp.");

    if ssh_result.success() {
        Ok(())
    } else {
        Err(format!("scp failed with error {:?}", ssh_result))
    }
}

pub fn ssh(verbose: bool, command: &str) {
    let netaddr_result = netaddr(verbose);
    let fuchsia_root = fuchsia_root();
    let ssh_config = fuchsia_root.join("out/debug-x86-64/ssh-keys/ssh_config");
    match netaddr_result {
        Ok(netaddr) => {
            let ssh_result = Command::new("ssh")
                .arg("-q")
                .arg("-F")
                .arg(ssh_config)
                .arg(netaddr)
                .arg(command)
                .status()
                .expect("Unable to run ssh.");
            if !ssh_result.success() {
                println!("ssh failed: {}", ssh_result);
            }

        }

        Err(netaddr_err) => {
            println!("{}", netaddr_err);
        }
    }
}

pub fn start_emulator(with_graphics: bool) {
    let fuchsia_root = fuchsia_root();
    let run_magenta_script = fuchsia_root.join("scripts/run-magenta-x86-64");
    let user_bootfs = fuchsia_root.join("out/debug-x86-64/user.bootfs");
    let user_bootfs_str = user_bootfs.to_str().unwrap();
    let mut args = vec!["-N", "-x", user_bootfs_str];
    if with_graphics {
        args.push("-g");
    }
    let child = Command::new(run_magenta_script)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Unable to run magenta.");
    println!("emulator started with process ID {}", child.id());

    if is_mac() {

        let user = env::var("USER").unwrap();

        println!("Calling sudo ifconfig to bring up tap0 interface; password may be required.");

        let chown_status = Command::new("sudo")
            .arg("chown")
            .arg(user)
            .arg("/dev/tap0")
            .status()
            .expect("Couldn't run chown.");

        if chown_status.success() {
            let ifconfig_status = Command::new("sudo")
                .arg("ifconfig")
                .arg("tap0")
                .arg("inet6")
                .arg("fc00::/7")
                .arg("up")
                .status()
                .expect("Couldn't run ifconfig.");

            if ifconfig_status.success() {
                println!("tap0 enabled");
                // If sudo needed a password, sometimes the terminal gets into a funky state
                Command::new("stty").arg("sane").status().expect("Couldn run stty");
            } else {
                println!("ifconfig failed");
            }
        }
    }
}

pub fn stop_emulator() {
    let _ = Command::new("killall")
        .arg("qemu-system-x86_64")
        .status()
        .expect("Could not run killall.");
}
