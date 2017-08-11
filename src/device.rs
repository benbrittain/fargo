// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::path::{PathBuf, Path};
use std::process::{Command, Stdio};
use std::{str, thread, time};
use sdk::{fuchsia_root, target_out_dir, TargetOptions};
use utils::is_mac;

error_chain!{
    foreign_links {
        Io(::std::io::Error);
    }
    links {
        SDK(::sdk::Error, ::sdk::ErrorKind);
    }
}

pub fn netaddr(verbose: bool) -> Result<String> {
    let fuchsia_root = fuchsia_root()?;
    let netaddr_binary = fuchsia_root.join("out/build-magenta/tools/netaddr");
    let netaddr_result = Command::new(netaddr_binary).arg("--fuchsia").output()?;
    let result = str::from_utf8(&netaddr_result.stdout)
        .unwrap()
        .trim()
        .to_string();
    if verbose {
        println!("netaddr result = {}", result);
    }
    Ok(result)
}

pub fn scp_to_device(
    verbose: bool,
    target_options: &TargetOptions,
    netaddr: &str,
    source_path: &PathBuf,
    destination_path: &str,
) -> Result<()> {
    let destination_with_address = format!("[{}]:{}", netaddr, destination_path);
    let ssh_config = target_out_dir(&target_options)?.join("ssh-keys/ssh_config");
    if !ssh_config.exists() {
        bail!("ssh config not found at {:?}", ssh_config);
    }
    if verbose {
        println!("destination_with_address = {}", destination_with_address);
        println!("ssh_config = {:?}", ssh_config);
    }

    let mut scp_command = Command::new("scp");

    scp_command
        .env_remove("SSH_AUTH_SOCK")
        .arg(if verbose { "-v" } else { "-q" })
        .arg("-F")
        .arg(ssh_config)
        .args(
            &[
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-o",
                "StrictHostKeyChecking=no",
            ],
        )
        .arg(source_path)
        .arg(destination_with_address);

    if verbose {
        println!("{:?}", scp_command);
    }

    let scp_result = scp_command.status().chain_err(|| "unable to run scp")?;

    if !scp_result.success() {
        bail!("scp failed with error {:?}", scp_result);
    }

    Ok(())
}

pub fn ssh(verbose: bool, target_options: &TargetOptions, command: &str) -> Result<()> {
    let netaddr = netaddr(verbose)?;
    let ssh_config = target_out_dir(&target_options)?.join("ssh-keys/ssh_config");
    if !ssh_config.exists() {
        bail!("ssh config not found at {:?}", ssh_config);
    }
    let ssh_result = Command::new("ssh")
        .env_remove("SSH_AUTH_SOCK")
        .arg("-q")
        .arg("-F")
        .arg(ssh_config)
        .args(
            &[
                "-o",
                "UserKnownHostsFile=/dev/null",
                "-o",
                "StrictHostKeyChecking=no",
            ],
        )
        .arg(netaddr)
        .arg(command)
        .status()
        .chain_err(|| "unable to run ssh")?;

    if !ssh_result.success() {
        bail!("ssh failed: {}", ssh_result);
    }

    Ok(())
}

pub fn setup_network_mac(user: &str) -> Result<()> {
    println!("Calling sudo ifconfig to bring up tap0 interface; password may be required.");

    let chown_status = Command::new("sudo")
        .arg("chown")
        .arg(user)
        .arg("/dev/tap0")
        .status()
        .chain_err(|| "couldn't run chown")?;

    if !chown_status.success() {
        bail!("chown failed: {}", chown_status);
    }

    let mut loop_count = 0;
    loop {
        let ifconfig_status = Command::new("sudo")
            .arg("ifconfig")
            .arg("tap0")
            .arg("inet6")
            .arg("fc00::/7")
            .arg("up")
            .status()
            .chain_err(|| "couldn't run ifconfig")?;

        if !ifconfig_status.success() {
            if loop_count > 10 {
                bail!("ifconfig failed: {}", ifconfig_status);
            }
            loop_count += 1;
            thread::sleep(time::Duration::from_millis(100));
        } else {
            break;
        }
    }

    println!("tap0 enabled");

    Command::new("stty").arg("sane").status().chain_err(
        || "couldn't run stty",
    )?;

    Ok(())
}

#[cfg_attr(rustfmt, rustfmt_skip)]
static TUNCTL_NOT_FOUND_ERROR: &'static str =
"tunctl command not found. Please install uml-utilities.
For help see https://fuchsia.googlesource.com/magenta/+/
master/docs/qemu.md#Enabling-Networking-under-QEMU-x86_64-only";

pub fn setup_network_linux(user: &str) -> Result<()> {
    // Create the tap network device if it doesn't exist.
    if !Path::new("/sys/class/net/qemu").exists() {
        println!(
            "Qemu tap device not found. Using sudo and tunctl to create \
            tap network device; password may be required."
        );
        let tunctl_status = Command::new("sudo")
            .args(&["tunctl", "-b", "-u", &user, "-t", "qemu"])
            .stdout(Stdio::null())
            .status()
            .map_err(|e| if e.kind() == ::std::io::ErrorKind::NotFound {
                Error::with_chain(e, TUNCTL_NOT_FOUND_ERROR)
            } else {
                Error::with_chain(e, "tunctl failed to create a new tap network device")
            })?;

        if !tunctl_status.success() {
            bail!("tunctl failed to create tap network device.");
        }
    }

    let ifconfig_status = Command::new("sudo")
        .arg("ifconfig")
        .arg("qemu")
        .arg("up")
        .status()
        .chain_err(|| "couldn't run ifconfig")?;

    if !ifconfig_status.success() {
        bail!("ifconfig failed");
    }

    Ok(())
}

pub fn setup_network() -> Result<()> {
    let user = env::var("USER").chain_err(|| "No $USER env var found.")?;
    if is_mac() {
        setup_network_mac(&user)
    } else {
        setup_network_linux(&user)
    }
}

pub fn start_emulator(
    with_graphics: bool,
    with_networking: bool,
    target_options: &TargetOptions,
) -> Result<()> {
    let fuchsia_root = fuchsia_root()?;
    let run_magenta_script = fuchsia_root.join("scripts/run-magenta-x86-64");
    if !run_magenta_script.exists() {
        bail!("run magenta script not found at {:?}", run_magenta_script);
    }
    let user_bootfs = target_out_dir(&target_options)?.join("user.bootfs");
    if !user_bootfs.exists() {
        bail!("user.bootfs not found at {:?}", user_bootfs);
    }
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
        .chain_err(|| "unable to run magenta")?;

    println!("emulator started with process ID {}", child.id());

    if with_networking {
        setup_network()
    } else {
        Ok(())
    }
}

pub fn stop_emulator() -> Result<()> {
    Command::new("killall").arg("qemu-system-x86_64").status()?;
    Ok(())
}
