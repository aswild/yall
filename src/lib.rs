//! yall: Yet Another Little Logger implementation
//!
//! Satisfies what I need for console applications, that is:
//!   * minimal dependencies
//!   * log to stderr
//!   * simple standard ANSI terminal colors
//!   * filename and line number for debug/trace logs
//!   * no color or log-level prefix for Info, treating that as normal output

use ansi_term::Color::{Blue, Cyan, Red, Yellow};
use atty::{is as isatty, Stream};
use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

/// The main struct of this crate which implements the [`Log`] trait.
///
/// Create one with `with_level` or `with_verbosity` and then call `init` or `try_init` on it.
///
/// [`Log`]: https://docs.rs/log/0.4.11/log/trait.Log.html
#[derive(Debug)]
pub struct Logger {
    level: LevelFilter,
    color: bool,
}

/// Color mode, the usual suspects. Auto is based on whether stderr is a TTY.
#[derive(Debug)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[inline]
fn auto_color() -> bool {
    isatty(Stream::Stderr)
}

impl Logger {
    /// Create a Logger with the given level.
    pub fn with_level(level: LevelFilter) -> Self {
        Self {
            level,
            color: auto_color(),
        }
    }

    /// Create a Logger with the given "verbosity" number. Useful for translating from
    /// the number of -v flags on the command line.
    ///
    /// 0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5+ = Trace
    pub fn with_verbosity(level: usize) -> Self {
        Self {
            level: match level {
                0 => LevelFilter::Off,
                1 => LevelFilter::Error,
                2 => LevelFilter::Warn,
                3 => LevelFilter::Info,
                4 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            },
            color: auto_color(),
        }
    }

    /// Sets the color mode to Auto/Always/Never. If you don't call this, the default
    /// mode is automatic based on whether stderr is a TTY.
    ///
    /// Returns `&mut self` so that this function can be used in builder-like syntax.
    pub fn color(&mut self, c: ColorMode) -> &mut Self {
        self.color = match c {
            ColorMode::Auto => auto_color(),
            ColorMode::Always => true,
            ColorMode::Never => false,
        };
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
}

impl Log for Logger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= self.level
    }

    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }

        let (style, msg) = match r.level() {
            Level::Error => (Some(Red.bold()), format!("[ERROR] {}", r.args())),
            Level::Warn => (Some(Yellow.bold()), format!("[WARN] {}", r.args())),
            Level::Info => (None, r.args().to_string()),
            Level::Debug => (
                Some(Cyan.normal()),
                format!(
                    "[DEBUG][{}:{}] {}",
                    r.file().unwrap_or("?"),
                    r.line().unwrap_or(0),
                    r.args()
                ),
            ),
            Level::Trace => (
                Some(Blue.normal()),
                format!(
                    "[TRACE][{}:{}] {}",
                    r.file().unwrap_or("?"),
                    r.line().unwrap_or(0),
                    r.args()
                ),
            ),
        };

        if self.color && style.is_some() {
            eprintln!("{}", style.unwrap().paint(msg));
        } else {
            eprintln!("{}", msg);
        }
    }

    fn flush(&self) {}
}
