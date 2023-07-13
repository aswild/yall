// Copyright (c) 2020 Allen Wild
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Demo of using yall::Logger with a basic `clap` app

use clap::{value_parser, Arg, ArgAction};
use yall::log_macros::*;
use yall::Logger;

fn main() {
    let args = clap::command!("yall example")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count)
                .help("be more verbose"),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .action(ArgAction::Count)
                .conflicts_with("verbose")
                .help("be more quiet"),
        )
        .arg(
            Arg::new("level")
                .short('l')
                .long("level")
                .value_parser(value_parser!(u64))
                .conflicts_with_all(["verbose", "quiet"])
                .help("set the numberic level, from 0=off to 5=trace"),
        )
        .arg(
            Arg::new("filename")
                .short('F')
                .long("full-filename")
                .action(ArgAction::SetTrue)
                .help("Show the full non-abbreviated filename in debug/trace logs"),
        )
        .get_matches();

    // create and register the logger. Most applications wouldn't have all three arguments used in
    // this example. Here, clap is set up so that verbose, level, and quiet are all
    // mutually-exclusive.
    let mut logger = if let Some(level) = args.get_one::<u64>("level") {
        Logger::with_verbosity(*level)
    } else {
        // can easily chain verbose and quiet adjustments
        Logger::new()
            .verbose(args.get_count("verbose") as u64)
            .quiet(args.get_count("quiet") as u64)
    };

    // Logger's builder methods pass by value instead of &mut, requiring the need to re-assign
    logger = logger.full_filename(args.get_flag("filename"));
    logger.init();

    trace!("loud noises");
    debug!("squash these bugs");
    info!("some normal information");
    warn!("oh deer");
    error!("gosh heckie");
}
