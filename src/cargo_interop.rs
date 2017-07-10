// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Structs for deserializing the output of cargo commands
// Derived from inspection of the output of cargo commands
// run with the "--message-format json" parameter.

#[derive(Deserialize,Debug)]
pub struct Profile {
    pub test: bool,
}

#[derive(Deserialize,Debug)]
pub struct Target {
    pub kind: Vec<String>,
}

#[derive(Deserialize,Debug)]
pub struct Artifact {
    #[serde(default)]
    pub filenames: Vec<String>,
    pub profile: Profile,
    pub target: Target,
}

#[derive(Deserialize,Debug)]
pub struct Code {
    code: Option<String>,
    explanation: String,
}

#[derive(Deserialize,Debug)]
pub struct Span {
    file_name: String,
    label: Option<String>,
    line_start: i32,
    line_end: i32,
}

#[derive(Deserialize,Debug)]
pub struct Message {
    level: String,
    message: String,
    code: Option<Code>,
    spans: Vec<Span>,
}

#[derive(Deserialize,Debug)]
pub struct MessageWrapper {
    message: Option<Message>,
}
