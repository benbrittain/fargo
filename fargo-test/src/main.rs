// Copyright 2017 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

extern crate fuchsia_zircon as zircon;

fn main() {
    println!("Hello, world!");
    println!("Hello, world!");
    println!("Hello, world!");
    println!("Hello, world!");
    println!("Hello, world!");
    println!("Hello, world!");
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {

    use zircon::{Channel, ChannelOpts};

    #[test]
    fn noop_test() {}

    #[test]
    fn channel_call_test() {
        // Create a pair of channels
        let (p1, p2) = Channel::create(ChannelOpts::Normal).unwrap();
    }
}
