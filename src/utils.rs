// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use failure::{Error, ResultExt};
use sdk::{TargetOptions, strip_tool_path};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use uname::uname;

#[allow(dead_code)]
pub fn duration_as_milliseconds(duration: &Duration) -> u64 {
    let subsec_ms: u64 = u64::from(duration.subsec_nanos()) / 1_000_000;
    duration.as_secs() * 1000 + subsec_ms
}

pub fn is_mac() -> bool {
    uname().unwrap().sysname == "Darwin"
}

pub fn strip_binary(binary: &PathBuf, target_options: &TargetOptions) -> Result<PathBuf, Error> {
    let file_name = binary.file_name().unwrap();
    let new_file_name = file_name.to_string_lossy().into_owned() + "_stripped";
    let target_path = binary.parent().unwrap().join(new_file_name);
    let strip_result = Command::new(strip_tool_path(target_options)?)
        .arg("-strip-all")
        .arg(binary)
        .arg(&target_path)
        .status()
        .context("strip command failed to start")?;

    if !strip_result.success() {
        bail!("strip failed with error {:?}", strip_result);
    }

    Ok(target_path)
}
