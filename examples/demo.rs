//! Demo of using yall::Logger with a basic `clap` app

use clap::{App, Arg};
use yall::log_macros::*;
use yall::Logger;

fn main() {
    let args = App::new("yall example")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("be more verbose"),
        )
        .arg(Arg::with_name("quiet").short("q").long("quiet").help("be quiet"))
        .arg(
            Arg::with_name("level")
                .short("l")
                .long("level")
                .takes_value(true)
                .help("set the numberic level, from 0=off to 5=trace"),
        )
        .get_matches();

    // default level is Info, plus however many times -v was passed
    let mut level: usize = 3 + (args.occurrences_of("verbose") as usize);

    // -q overrides any -v
    if args.is_present("quiet") {
        level = 0;
    }

    // -l overrides -q and -v
    if let Some(l) = args.value_of("level") {
        level = l.parse().expect("invalid level number");
    }

    // create and register the logger
    Logger::with_verbosity(level).init();

    trace!("loud noises");
    debug!("squash these bugs");
    info!("some normal information");
    warn!("oh deer");
    error!("gosh heckie");
}
