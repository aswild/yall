// Copyright (c) 2020 Allen Wild
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # yall: Yet Another Little Logger
//!
//! A simple lightweight backend for the [`log`](::log) crate.
//!
//!   * Logs to stderr
//!   * Simple standard terminal colors, no RGB or 256-color themes that may clash with the
//!     terminal theme
//!   * By default, color is auto-detected based on whether stderr is a tty, but can be forced
//!     on or off with the [`Logger::color`] method.
//!   * Info level messages are unformatted with no color or prefix
//!   * Error/Warn/Debug/Trace messages are Red/Yellow/Cyan/Blue, respectively
//!   * Debug and Trace levels show the filename and line number.
//!   * Minimal dependencies
//!   * Configured with code rather than environment variables

use std::fmt;
use std::io::{self, Write};
use std::sync::Mutex;

use is_terminal::IsTerminal;
use log::{Level, Log, Metadata, Record, SetLoggerError};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[doc(no_inline)]
pub use log::LevelFilter;

/// Re-exports of the error, warn, info, debug, and trace macros in the log crate.
///
/// Convenient for glob-importing with `use yall::log_macros::*;`
pub mod log_macros {
    #[doc(no_inline)]
    pub use log::{debug, error, info, trace, warn};
}

/// Whether to enable colored output, the usual suspects.
#[derive(Debug)]
pub enum ColorMode {
    /// Enable color automatically if stderr is a tty, plus the `TERM` and `NO_COLOR` environment
    /// variable checks done by `termcolor`'s [`ColorChoice::Auto`] variant.
    Auto,
    /// Always enable colored output.
    Always,
    /// Never enable colored output.
    Never,
}

impl ColorMode {
    /// Internal function to map ColorMode to a termcolor::ColorChoice that Logger uses internally.
    /// This is mainly to keep termcolor out of yall's API.
    fn to_color_choice(&self) -> ColorChoice {
        match self {
            ColorMode::Auto => {
                if io::stderr().is_terminal() {
                    // termcolor will check for TERM and NO_COLOR when creating a StandardStream
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            }
            ColorMode::Always => ColorChoice::Always,
            ColorMode::Never => ColorChoice::Never,
        }
    }
}

impl Default for ColorMode {
    /// The default ColorMode is `Auto`
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug)]
struct LogColors {
    error: ColorSpec,
    warn: ColorSpec,
    info: ColorSpec,
    debug: ColorSpec,
    trace: ColorSpec,
}

impl LogColors {
    pub fn new() -> Self {
        // The set_* functions return &mut, so we need to_owned() to convert back to an actual
        // value. Since ColorSpec doesn't implement Copy, we can't just dereference.
        let error = ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).to_owned();
        let warn = ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true).to_owned();
        let info = ColorSpec::new();
        let debug = ColorSpec::new().set_fg(Some(Color::Cyan)).to_owned();
        let trace = ColorSpec::new().set_fg(Some(Color::Blue)).to_owned();

        Self { error, warn, info, debug, trace }
    }

    pub fn get(&self, l: Level) -> &ColorSpec {
        match l {
            Level::Error => &self.error,
            Level::Warn => &self.warn,
            Level::Info => &self.info,
            Level::Debug => &self.debug,
            Level::Trace => &self.trace,
        }
    }
}

/// Internal extension trait for working with log::LevelFilter as an integer. Since LevelFilter is
/// Copy, all these methods take self by value to avoid unnecessary pointers.
trait LevelFilterExt {
    fn from_int(val: u8) -> Self;
    fn to_int(self) -> u8;
    fn add(self, change: u8) -> Self;
    fn sub(self, change: u8) -> Self;
}

// LevelFilter is Copy and repr(usize) and the match blocks here are the same as LevelFilter's
// discriminant order, so they compile down to almost nothing. from_int is one branch for val>5,
// to_int is a single move that basically gets inlined into a nop.
// Not that this is hot code anyway...
impl LevelFilterExt for LevelFilter {
    fn from_int(val: u8) -> Self {
        match val {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    }

    fn to_int(self) -> u8 {
        match self {
            LevelFilter::Off => 0,
            LevelFilter::Error => 1,
            LevelFilter::Warn => 2,
            LevelFilter::Info => 3,
            LevelFilter::Debug => 4,
            LevelFilter::Trace => 5,
        }
    }

    fn add(self, change: u8) -> Self {
        Self::from_int(self.to_int().saturating_add(change))
    }

    fn sub(self, change: u8) -> Self {
        Self::from_int(self.to_int().saturating_sub(change))
    }
}

/// The main struct of this crate which implements the [`Log`] trait.
///
/// Create one using [`with_level`](Self::with_level) or
/// [`with_verbosity`](Self::with_verbosity) and then call [`init`](Self::init) or
/// [`try_init`](Self::try_init) on it.
pub struct Logger {
    level: LevelFilter,
    colors: LogColors,
    use_full_filename: bool,
    out: Mutex<StandardStream>,
}

// StandardStream doesn't impl Debug, so we can't derive it. Instead do this manual implementation
// with a dummy value for out.
impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Logger")
            .field("level", &self.level)
            .field("colors", &self.colors)
            .field("use_full_filename", &self.use_full_filename)
            .field("out", &"Mutex<termcolor::StandardStream::stderr>")
            .finish()
    }
}

impl Default for Logger {
    /// Create a Logger with the default Info level
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    /// Create a Logger with the default Info level
    pub fn new() -> Logger {
        Self::with_level(LevelFilter::Info)
    }

    /// Create a Logger with the given level.
    pub fn with_level(level: LevelFilter) -> Logger {
        Self {
            level,
            colors: LogColors::new(),
            use_full_filename: false,
            out: Mutex::new(StandardStream::stderr(ColorMode::default().to_color_choice())),
        }
    }

    /// Create a Logger with the given "verbosity" number. Useful for translating a number of -v
    /// flags in command-line arguments.
    ///
    /// 0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5+ = Trace
    pub fn with_verbosity(level: u8) -> Logger {
        Self::with_level(LevelFilter::from_int(level))
    }

    /// Increase the verbosity level by the amount given. Takes a `u8` as returned by
    /// `clap::ArgMatches::get_count`.
    pub fn verbose(mut self, change: u8) -> Logger {
        self.level = self.level.add(change);
        self
    }

    /// Decrease the verbosity level by the amount given. Takes a `u8` as returned by
    /// `clap::ArgMatches::get_count`.
    pub fn quiet(mut self, change: u8) -> Logger {
        self.level = self.level.sub(change);
        self
    }

    /// Sets the color mode, see [`ColorMode`] for details.
    pub fn color(mut self, c: ColorMode) -> Logger {
        // we can't change the ColorChoice of a StandardStream, but we can just re-create it
        self.out = Mutex::new(StandardStream::stderr(c.to_color_choice()));
        self
    }

    /// By default, yall will shorten the filename displayed in Debug and Trace logs by removing
    /// a "src/" prefix and ".rs" suffix, if present. Use this function to disable that and print
    /// the full unchanged filename.
    pub fn full_filename(mut self, full: bool) -> Logger {
        self.use_full_filename = full;
        self
    }

    /// Register this as the global logger with the [`log`](::log) crate. May fail if the application has
    /// already set a logger.
    pub fn try_init(self) -> Result<(), SetLoggerError> {
        log::set_max_level(self.level);
        log::set_boxed_logger(Box::new(self))
    }

    /// Same as [`try_init`](Self::try_init) but panic on failure.
    pub fn init(self) {
        self.try_init().expect("failed to initialize logger");
    }

    /// Internal wrapper function for the meat of the logging that returns a Result, in case the
    /// termcolors printing fails somehow. Assumes that we've already checked that the record's
    /// log level is in fact enabled.
    fn print_log(&self, r: &Record) -> io::Result<()> {
        let level = r.level();

        // strip "src/" prefix and ".rs" suffix
        let mut filename = r.file().unwrap_or("?");
        if !self.use_full_filename && (level == Level::Debug || level == Level::Trace) {
            // we could use str::strip_{prefix,suffix} here, but they're not stable until
            // rust 1.45 and return Options which is kinda clunky.
            if filename.starts_with("src/") {
                filename = &filename[4..];
            }
            if filename.ends_with(".rs") {
                filename = &filename[..(filename.len() - 3)];
            }
        }

        let mut out = self.out.lock().unwrap();
        out.set_color(self.colors.get(level))?;
        match level {
            Level::Error => writeln!(out, "[ERROR] {}", r.args()),
            Level::Warn => writeln!(out, "[WARN] {}", r.args()),
            Level::Info => writeln!(out, "{}", r.args()),
            Level::Debug => {
                writeln!(out, "[DEBUG][{}:{}] {}", filename, r.line().unwrap_or(0), r.args())
            }
            Level::Trace => {
                writeln!(out, "[TRACE][{}:{}] {}", filename, r.line().unwrap_or(0), r.args())
            }
        }?;
        out.reset()?;
        Ok(())
    }
}

impl Log for Logger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= self.level
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }

        if let Err(e) = self.print_log(r) {
            // uh oh, something in termcolor failed
            eprintln!("LOGGING ERROR: failed to write log message because of '{}'", e);
            eprintln!("Original message: {}: {}", r.level(), r.args());
        }
    }

    fn flush(&self) {
        let mut out = self.out.lock().unwrap();
        let _ = out.flush();
    }
}
