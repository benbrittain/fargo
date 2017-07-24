// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str;
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
    let netaddr_result = Command::new(netaddr_binary).arg("--fuchsia")
        .output()?;
    let result = str::from_utf8(&netaddr_result.stdout).unwrap().trim().to_string();
    if verbose {
        println!("netaddr result = {}", result);
    }
    Ok(result)
}

pub fn scp_to_device(verbose: bool,
                     target_options: &TargetOptions,
                     netaddr: &String,
                     source_path: &PathBuf,
                     destination_path: &String)
                     -> Result<()> {
    let destination_with_address = format!("[{}]:{}", netaddr, destination_path);
    let ssh_config = target_out_dir(&target_options)?.join("ssh-keys/ssh_config");
    if !ssh_config.exists() {
        bail!("ssh config not found at {:?}", ssh_config);
    }
    let ssh_result = Command::new("scp").arg(if verbose { "-v" } else { "-q" })
        .arg("-F")
        .arg(ssh_config)
        .arg(source_path)
        .arg(destination_with_address)
        .status()
        .chain_err(|| "unable to run scp")?;

    if !ssh_result.success() {
        bail!("scp failed with error {:?}", ssh_result);
    }

    Ok(())
}

pub fn ssh(verbose: bool, target_options: &TargetOptions, command: &str) -> Result<()> {
    let netaddr = netaddr(verbose)?;
    let ssh_config = target_out_dir(&target_options)?.join("ssh-keys/ssh_config");
    if !ssh_config.exists() {
        bail!("ssh config not found at {:?}", ssh_config);
    }
    let ssh_result = Command::new("ssh").arg("-q")
        .arg("-F")
        .arg(ssh_config)
        .arg(netaddr)
        .arg(command)
        .status()
        .chain_err(|| "unable to run ssh")?;

    if !ssh_result.success() {
        bail!("ssh failed: {}", ssh_result);
    }

    Ok(())
}

pub fn start_emulator(with_graphics: bool, target_options: &TargetOptions) -> Result<()> {
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

    let child = Command::new(run_magenta_script).args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .chain_err(|| "unable to run magenta")?;

    println!("emulator started with process ID {}", child.id());

    if is_mac() {

        let user = env::var("USER").unwrap();

        // TODO; Poll for /dev/tap0 as it can take a while for the emulator
        //       to create it.

        println!("Calling sudo ifconfig to bring up tap0 interface; password may be required.");

        let chown_status = Command::new("sudo").arg("chown")
            .arg(user)
            .arg("/dev/tap0")
            .status()
            .chain_err(|| "couldn't run chown")?;

        if !chown_status.success() {
            bail!("chown failed: {}", chown_status);
        }

        let ifconfig_status = Command::new("sudo").arg("ifconfig")
            .arg("tap0")
            .arg("inet6")
            .arg("fc00::/7")
            .arg("up")
            .status()
            .chain_err(|| "couldn't run ifconfig")?;

        if !ifconfig_status.success() {
            bail!("ifconfig failed: {}", chown_status);
        }

        println!("tap0 enabled");

        Command::new("stty").arg("sane")
            .status()
            .chain_err(|| "couldn't run stty")?;

    }

    Ok(())
}

pub fn stop_emulator() -> Result<()> {
    Command::new("killall").arg("qemu-system-x86_64")
        .status()?;
    Ok(())
}
