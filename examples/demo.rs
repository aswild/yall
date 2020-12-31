// Copyright (c) 2020 Allen Wild
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Demo of using yall::Logger with a basic `clap` app

use clap::{App, Arg};
use yall::log_macros::*;
use yall::Logger;

fn main() {
    #[rustfmt::skip]
    let args = App::new("yall example")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("be more verbose")
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .multiple(true)
                .conflicts_with("verbose")
                .help("be more quiet")
        )
        .arg(
            Arg::with_name("level")
                .short("l")
                .long("level")
                .takes_value(true)
                .conflicts_with_all(&["verbose", "quiet"])
                .help("set the numberic level, from 0=off to 5=trace"),
        )
        .arg(
            Arg::with_name("filename")
                .short("F")
                .long("full-filename")
                .help("Show the full non-abbreviated filename in debug/trace logs"),
        )
        .get_matches();

    // create and register the logger. Most applications wouldn't have all three arguments used in
    // this example. Here, clap is set up so that verbose, level, and quiet are all
    // mutually-exclusive.
    let mut logger = if let Some(l) = args.value_of("level") {
        let level = l.parse().expect("invalid level number");
        Logger::with_verbosity(level)
    } else {
        // can easily chain verbose and quiet adjustments
        Logger::new().verbose(args.occurrences_of("verbose")).quiet(args.occurrences_of("quiet"))
    };

    // Logger's builder methods pass by value instead of &mut, requiring the need to re-assign
    logger = logger.full_filename(args.is_present("filename"));
    logger.init();

    trace!("loud noises");
    debug!("squash these bugs");
    info!("some normal information");
    warn!("oh deer");
    error!("gosh heckie");
}
