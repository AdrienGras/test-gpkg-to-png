//! Logging and verbosity control for the application.
//!
//! Provides a global logger with three verbosity levels:
//! - Quiet: Only errors and final results
//! - Normal: Progress bars and essential info (default)
//! - Verbose: Everything including debug details

use std::io::Write;
use std::sync::OnceLock;

/// Verbosity level for controlling output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    /// Only errors and final results
    Quiet,
    /// Progress bars and essential info (default)
    Normal,
    /// Everything including debug details
    Verbose,
}

/// Global logger instance.
static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Thread-safe logger for controlling application output.
#[derive(Debug)]
pub struct Logger {
    level: VerbosityLevel,
}

impl Logger {
    /// Initialize the global logger with the specified verbosity level.
    ///
    /// # Panics
    /// Panics if called more than once.
    pub fn init(level: VerbosityLevel) {
        LOGGER
            .set(Logger { level })
            .expect("Logger already initialized");
    }

    /// Get the global logger instance.
    ///
    /// # Panics
    /// Panics if the logger hasn't been initialized.
    pub fn instance() -> &'static Logger {
        LOGGER.get().expect("Logger not initialized")
    }

    /// Check if the current level is at least the given level.
    fn is_at_least(&self, level: VerbosityLevel) -> bool {
        let current = self.level as i32;
        let required = level as i32;
        current >= required
    }

    /// Returns true if verbose mode is enabled.
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

    /// Log an error message (always displayed).
    pub fn error(&self, msg: &str) {
        eprintln!("Error: {}", msg);
    }

    /// Log a success message (displayed in quiet mode and above).
    pub fn success(&self, msg: &str) {
        if self.is_at_least(VerbosityLevel::Quiet) {
            println!("{}", msg);
        }
    }

    /// Log an info message (displayed in normal mode and above).
    pub fn info(&self, msg: &str) {
        if self.is_at_least(VerbosityLevel::Normal) {
            println!("{}", msg);
        }
    }

    /// Log a debug message (displayed only in verbose mode).
    pub fn debug(&self, msg: &str) {
        if self.is_at_least(VerbosityLevel::Verbose) {
            eprintln!("[DEBUG] {}", msg);
        }
    }

    /// Write a message directly to stdout without newline (for progress bars).
    /// Only writes in normal mode and above.
    #[allow(dead_code)]
    pub fn write(&self, msg: &str) {
        if self.is_at_least(VerbosityLevel::Normal) {
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

/// Log a success message (displayed in quiet mode and above).
pub fn success(msg: &str) {
    Logger::instance().success(msg);
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

    fn reset_logger() {
        // Note: In tests, we can't actually reset OnceLock, so we test directly on Logger instances
    }

    #[test]
    fn test_verbosity_level_ordering() {
        // Ensure the enum values are ordered correctly for comparison
        assert!((VerbosityLevel::Quiet as i32) < (VerbosityLevel::Normal as i32));
        assert!((VerbosityLevel::Normal as i32) < (VerbosityLevel::Verbose as i32));
    }

    #[test]
    fn test_logger_is_verbose() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
        };
        let normal_logger = Logger {
            level: VerbosityLevel::Normal,
        };
        let verbose_logger = Logger {
            level: VerbosityLevel::Verbose,
        };

        assert!(!quiet_logger.is_verbose());
        assert!(!normal_logger.is_verbose());
        assert!(verbose_logger.is_verbose());
    }

    #[test]
    fn test_logger_is_quiet() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
        };
        let normal_logger = Logger {
            level: VerbosityLevel::Normal,
        };
        let verbose_logger = Logger {
            level: VerbosityLevel::Verbose,
        };

        assert!(quiet_logger.is_quiet());
        assert!(!normal_logger.is_quiet());
        assert!(!verbose_logger.is_quiet());
    }

    #[test]
    fn test_logger_level() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
        };

        assert_eq!(quiet_logger.level(), VerbosityLevel::Quiet);
    }

    #[test]
    fn test_is_at_least() {
        let quiet_logger = Logger {
            level: VerbosityLevel::Quiet,
        };
        let normal_logger = Logger {
            level: VerbosityLevel::Normal,
        };
        let verbose_logger = Logger {
            level: VerbosityLevel::Verbose,
        };

        // Quiet logger: only at least Quiet
        assert!(quiet_logger.is_at_least(VerbosityLevel::Quiet));
        assert!(!quiet_logger.is_at_least(VerbosityLevel::Normal));
        assert!(!quiet_logger.is_at_least(VerbosityLevel::Verbose));

        // Normal logger: at least Quiet and Normal
        assert!(normal_logger.is_at_least(VerbosityLevel::Quiet));
        assert!(normal_logger.is_at_least(VerbosityLevel::Normal));
        assert!(!normal_logger.is_at_least(VerbosityLevel::Verbose));

        // Verbose logger: at least all levels
        assert!(verbose_logger.is_at_least(VerbosityLevel::Quiet));
        assert!(verbose_logger.is_at_least(VerbosityLevel::Normal));
        assert!(verbose_logger.is_at_least(VerbosityLevel::Verbose));
    }
}
