//! # yall: Yet Another Little Logger implementation
//!
//! Satisfies what I need for console applications, that is:
//!   * minimal dependencies
//!   * log to stderr
//!   * simple standard terminal colors
//!   * filename and line number for debug/trace logs, but not other levels
//!   * no color or log-level prefix for Info, treating that as normal output

use std::io::Write;

use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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
    /// enable color unless stderr is not a TTY, `TERM` in the environment is empty or set to
    /// `dumb`, or if `NO_COLOR` is set in the environment.
    Auto,
    /// never enable colored output
    Always,
    /// always enabled colored output
    Never,
}

impl ColorMode {
    /// internal function to map ColorMode to a termcolor::ColorChoice that Logger uses internally.
    /// This is mainly to keep termcolor out of yall's API.
    fn to_color_choice(&self) -> ColorChoice {
        match self {
            ColorMode::Auto => {
                if atty::is(atty::Stream::Stderr) {
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
    /// the default ColorMode is `Auto`
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
        let mut error = ColorSpec::new();
        let mut warn = ColorSpec::new();
        let info = ColorSpec::new();
        let mut debug = ColorSpec::new();
        let mut trace = ColorSpec::new();

        error.set_fg(Some(Color::Red)).set_bold(true);
        warn.set_fg(Some(Color::Yellow)).set_bold(true);
        debug.set_fg(Some(Color::Cyan));
        trace.set_fg(Some(Color::Blue));

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

/// The main struct of this crate which implements the [`Log`] trait.
///
/// Create one with `with_level` or `with_verbosity` and then call `init` or `try_init` on it.
///
/// [`Log`]: https://docs.rs/log/0.4.11/log/trait.Log.html
#[derive(Debug)]
pub struct Logger {
    level: LevelFilter,
    color_choice: ColorChoice,
    colors: LogColors,
}

impl Logger {
    /// Create a Logger with the given level.
    pub fn with_level(level: LevelFilter) -> Self {
        Self {
            level,
            color_choice: ColorMode::default().to_color_choice(),
            colors: LogColors::new(),
        }
    }

    /// Create a Logger with the given "verbosity" number. Useful for translating from
    /// the number of -v flags on the command line.
    ///
    /// 0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5+ = Trace
    pub fn with_verbosity(level: usize) -> Self {
        Self::with_level(match level {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
    }

    /// Sets the color mode, see [`ColorMode`] for details.
    ///
    /// Returns `&mut self` so that this function can be used with builder syntax.
    ///
    /// [`ColorMode`]: enum.ColorMode.html
    pub fn color(&mut self, c: ColorMode) -> &mut Self {
        self.color_choice = c.to_color_choice();
        self
    }

    /// Register this as the global logger with the [`log`] crate.
    /// May fail if the application has already set a logger.
    ///
    /// [`log`]: https://docs.rs/log/latest/log/
    pub fn try_init(self) -> Result<(), SetLoggerError> {
        log::set_max_level(self.level);
        log::set_boxed_logger(Box::new(self))
    }

    /// Same as `try_init` but panic on failure.
    pub fn init(self) {
        self.try_init().expect("failed to initialize logger");
    }

    /// wrapper function for the meat of the logging that returns a Result, in case
    /// somehow the termcolors printing fails.
    fn print_log(&self, out: &mut StandardStream, r: &Record) -> std::io::Result<()> {
        let level = r.level();
        out.set_color(self.colors.get(level))?;
        match level {
            Level::Error => writeln!(out, "[ERROR] {}", r.args()),
            Level::Warn => writeln!(out, "[WARN] {}", r.args()),
            Level::Info => writeln!(out, "{}", r.args()),
            Level::Debug => writeln!(
                out,
                "[DEBUG][{}:{}] {}",
                r.file().unwrap_or("?"),
                r.line().unwrap_or(0),
                r.args()
            ),
            Level::Trace => writeln!(
                out,
                "[TRACE][{}:{}] {}",
                r.file().unwrap_or("?"),
                r.line().unwrap_or(0),
                r.args()
            ),
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

        // The termcolors output stream must be mut but log takes &self, so we have to reinitialize
        // it every time. Even with eprintln! there's still probably internal creation of io::stderr()
        // so hopefully this isn't too much overhead.
        let mut out = StandardStream::stderr(self.color_choice);

        if let Err(e) = self.print_log(&mut out, r) {
            // uh oh, something in termcolor failed
            eprintln!("LOGGING ERROR: failed to write log message because of '{}'", e);
            eprintln!("Original message: {}: {}", r.level(), r.args());
        }
    }

    fn flush(&self) {}
}
