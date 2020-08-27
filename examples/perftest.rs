// Copyright (c) 2020 Allen Wild
// SPDX-License-Identifier: MIT OR Apache-2.0

use yall::log_macros::*;
use yall::Logger;

fn main() {
    let count: usize = match std::env::args().nth(1) {
        Some(c) => c.parse().unwrap(),
        None => 100,
    };

    Logger::new().init();

    for i in 1..=count {
        info!("info log {}", i);
    }
}
