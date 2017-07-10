// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;
use std::process::Command;
use std::str;
use std::time::Duration;
use uname::uname;
use sdk::strip_tool_path;

#[allow(dead_code)]
pub fn duration_as_milliseconds(duration: &Duration) -> u64 {
    let subsec_ms: u64 = duration.subsec_nanos() as u64 / 1000000;
    let dur_ms = duration.as_secs() * 1000 + subsec_ms;
    dur_ms
}

pub fn is_mac() -> bool {
    uname().unwrap().sysname == "Darwin"
}

pub fn strip_binary(binary: &PathBuf) -> PathBuf {
    let file_name = binary.file_name().unwrap();
    let new_file_name = file_name.to_string_lossy().into_owned() + "_stripped";
    let target_path = binary.parent().unwrap().join(new_file_name);
    let strip_result = Command::new(strip_tool_path())
        .arg(binary)
        .arg("-o")
        .arg(&target_path)
        .status()
        .expect("strip command failed to start");

    if !strip_result.success() {
        panic!("strip failed with error {:?}", strip_result)
    }
    target_path
}
