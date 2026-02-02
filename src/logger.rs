//! Logging and verbosity control for the application.
//!
//! Provides a global logger with three verbosity levels:
//! - Quiet: Only file paths output
//! - Normal: Progress messages without prefixes (default)
//! - Verbose: Timestamped colored logs with details

use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

/// Verbosity level for controlling output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    /// Only file paths output
    Quiet,
    /// Progress messages without prefixes (default)
    Normal,
    /// Timestamped colored logs with details
    Verbose,
}

/// Global logger instance.
static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Global start time for elapsed calculations.
static START_TIME: OnceLock<Instant> = OnceLock::new();

/// Thread-safe logger for controlling application output.
#[derive(Debug)]
pub struct Logger {
    level: VerbosityLevel,
    colors_enabled: bool,
}

impl Logger {
    /// Initialize the global logger with the specified verbosity level.
    ///
    /// # Panics
    /// Panics if called more than once.
    pub fn init(level: VerbosityLevel, no_color: bool) {
        let colors_enabled = !no_color
            && std::env::var("NO_COLOR").is_err()
            && atty::is(atty::Stream::Stdout);

        START_TIME.set(Instant::now()).ok();
        LOGGER
            .set(Logger { level, colors_enabled })
            .expect("Logger already initialized");
    }

    /// Get the global logger instance.
    ///
    /// # Panics
    /// Panics if the logger hasn't been initialized.
    pub fn instance() -> &'static Logger {
        LOGGER.get().expect("Logger not initialized")
    }

    /// Get elapsed time since logger init.
    fn elapsed(&self) -> f64 {
        START_TIME
            .get()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Returns true if verbose mode is enabled.
    #[allow(dead_code)]
    pub fn is_verbose(&self) -> bool {
        self.level == VerbosityLevel::Verbose
    }

    /// Returns true if quiet mode is enabled.
    pub fn is_quiet(&self) -> bool {
        self.level == VerbosityLevel::Quiet
    }

    /// Returns the current verbosity level.
    #[allow(dead_code)]
    pub fn level(&self) -> VerbosityLevel {
        self.level
    }

    /// Log with level prefix and timestamp (verbose mode).
    fn log_with_level(&self, level: &str, msg: &str) {
        let elapsed = self.elapsed();
        if self.colors_enabled {
            let level_color = match level {
                "ERROR" => "\x1b[31m",
                "WARN" => "\x1b[33m",
                "INFO" => "\x1b[34m",
                "DEBUG" => "\x1b[90m",
                _ => "",
            };
            println!(
                "\x1b[90m[{:.2}s]\x1b[0m {}[{}]\x1b[0m {}",
                elapsed, level_color, level, msg
            );
        } else {
            println!("[{:.2}s] [{}] {}", elapsed, level, msg);
        }
    }

    /// Log an error message (always displayed).
    pub fn error(&self, msg: &str) {
        if self.level == VerbosityLevel::Verbose {
            let elapsed = self.elapsed();
            if self.colors_enabled {
                eprintln!(
                    "\x1b[90m[{:.2}s]\x1b[0m \x1b[31m[ERROR]\x1b[0m {}",
                    elapsed, msg
                );
            } else {
                eprintln!("[{:.2}s] [ERROR] {}", elapsed, msg);
            }
        } else {
            eprintln!("Error: {}", msg);
        }
    }

    /// Log a warning message (normal and verbose modes).
    pub fn warn(&self, msg: &str) {
        match self.level {
            VerbosityLevel::Quiet => {}
            VerbosityLevel::Normal => println!("{}", msg),
            VerbosityLevel::Verbose => self.log_with_level("WARN", msg),
        }
    }

    /// Output a file path (quiet: just path, normal: message, verbose: with prefix).
    pub fn output(&self, path: &str) {
        match self.level {
            VerbosityLevel::Quiet => println!("{}", path),
            VerbosityLevel::Normal => println!("Saved: {}", path),
            VerbosityLevel::Verbose => self.log_with_level("INFO", &format!("Saved: {}", path)),
        }
    }

    /// Log an info message (displayed in normal mode and above).
    pub fn info(&self, msg: &str) {
        match self.level {
            VerbosityLevel::Quiet => {}
            VerbosityLevel::Normal => println!("{}", msg),
            VerbosityLevel::Verbose => self.log_with_level("INFO", msg),
        }
    }

    /// Log a debug message (displayed only in verbose mode).
    pub fn debug(&self, msg: &str) {
        if self.level == VerbosityLevel::Verbose {
            self.log_with_level("DEBUG", msg);
        }
    }

    /// Write a message directly to stdout without newline (for progress bars).
    /// Only writes in normal mode.
    #[allow(dead_code)]
    pub fn write(&self, msg: &str) {
        if self.level == VerbosityLevel::Normal {
            print!("{}", msg);
            std::io::stdout().flush().ok();
        }
    }
}

/// Log an error message (always displayed).
#[allow(dead_code)]
pub fn error(msg: &str) {
    Logger::instance().error(msg);
}

/// Log a warning message (normal and verbose modes).
pub fn warn(msg: &str) {
    Logger::instance().warn(msg);
}

/// Output a file path.
pub fn output(path: &str) {
    Logger::instance().output(path);
}

/// Log an info message (displayed in normal mode and above).
pub fn info(msg: &str) {
    Logger::instance().info(msg);
}

/// Log a debug message (displayed only in verbose mode).
pub fn debug(msg: &str) {
    Logger::instance().debug(msg);
}

/// Returns true if verbose mode is enabled.
#[allow(dead_code)]
pub fn is_verbose() -> bool {
    Logger::instance().is_verbose()
}

/// Returns true if quiet mode is enabled.
#[allow(dead_code)]
pub fn is_quiet() -> bool {
    Logger::instance().is_quiet()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_level_ordering() {
        assert!((VerbosityLevel::Quiet as i32) < (VerbosityLevel::Normal as i32));
        assert!((VerbosityLevel::Normal as i32) < (VerbosityLevel::Verbose as i32));
    }

    #[test]
    fn test_logger_is_verbose() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
            colors_enabled: false,
        };
        let normal_logger = Logger {
            level: VerbosityLevel::Normal,
            colors_enabled: false,
        };
        let verbose_logger = Logger {
            level: VerbosityLevel::Verbose,
            colors_enabled: false,
        };

        assert!(!quiet_logger.is_verbose());
        assert!(!normal_logger.is_verbose());
        assert!(verbose_logger.is_verbose());
    }

    #[test]
    fn test_logger_is_quiet() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
            colors_enabled: false,
        };
        let normal_logger = Logger {
            level: VerbosityLevel::Normal,
            colors_enabled: false,
        };
        let verbose_logger = Logger {
            level: VerbosityLevel::Verbose,
            colors_enabled: false,
        };

        assert!(quiet_logger.is_quiet());
        assert!(!normal_logger.is_quiet());
        assert!(!verbose_logger.is_quiet());
    }

    #[test]
    fn test_logger_level() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
            colors_enabled: false,
        };

        assert_eq!(quiet_logger.level(), VerbosityLevel::Quiet);
    }

    #[test]
    fn test_elapsed_returns_value() {
        let logger = Logger {
            level: VerbosityLevel::Verbose,
            colors_enabled: false,
        };
        // Without START_TIME set, elapsed returns 0.0
        let elapsed = logger.elapsed();
        assert!(elapsed >= 0.0);
    }
}
