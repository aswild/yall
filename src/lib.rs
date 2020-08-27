//! # yall: Yet Another Little Logger
//!
//! A simple lightweight backend for the [`log`] crate.
//!
//!   * Logs to stderr
//!   * Simple standard terminal colors, no RGB or 256-color themes that may clash with the
//!     terminal theme
//!   * Info level messages are unformatted with no color or prefix
//!   * Error/Warn/Debug/Trace messages are Red/Yellow/Cyan/Blue, respectively
//!   * Debug and Trace levels show the filename and line number.
//!   * Minimal dependencies
//!   * Configured with code rather than environment variables
//!
//! [`log`]: https://docs.rs/log/latest

use std::io::{self, Write};

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
    /// Enable color automatically if stderr is a tty, plus the `TERM` and `NO_COLOR` environment
    /// variable checks done by `termcolor`'s [`ColorChoice::Auto`][ccauto] variant.
    ///
    /// [ccauto]: https://docs.rs/termcolor/latest/termcolor/enum.ColorChoice.html#variant.Auto
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

/// The main struct of this crate which implements the [`Log`] trait.
///
/// Create one using `with_level` or `with_verbosity` and then call `init` or `try_init` on it.
///
/// [`Log`]: https://docs.rs/log/latest/log/trait.Log.html
#[derive(Debug)]
pub struct Logger {
    level: LevelFilter,
    color_choice: ColorChoice,
    colors: LogColors,
}

impl Logger {
    /// Create a Logger with the given level.
    pub fn with_level(level: LevelFilter) -> Logger {
        Self {
            level,
            color_choice: ColorMode::default().to_color_choice(),
            colors: LogColors::new(),
        }
    }

    /// Create a Logger with the given "verbosity" number. Useful for translating a number of -v
    /// flags in command-line arguments.
    ///
    /// 0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5+ = Trace
    pub fn with_verbosity(level: usize) -> Logger {
        Self::with_level(match level {
            0 => LevelFilter::Off,
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        })
    }

    /// Sets the color mode, see [`ColorMode`](enum.ColorMode.html) for details.
    pub fn color(&mut self, c: ColorMode) -> &mut Logger {
        self.color_choice = c.to_color_choice();
        self
    }

    /// Register this as the global logger with the [`log`] crate.  May fail if the application has
    /// already set a logger.
    ///
    /// [`log`]: https://docs.rs/log/latest/
    pub fn try_init(self) -> Result<(), SetLoggerError> {
        log::set_max_level(self.level);
        log::set_boxed_logger(Box::new(self))
    }

    /// Same as `try_init` but panic on failure.
    pub fn init(self) {
        self.try_init().expect("failed to initialize logger");
    }

    /// Internal wrapper function for the meat of the logging that returns a Result, in case the
    /// termcolors printing fails somehow
    fn print_log(&self, out: &mut StandardStream, r: &Record) -> io::Result<()> {
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
