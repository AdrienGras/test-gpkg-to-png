# Verbosity Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor the logging system with three distinct modes: quiet (paths only), normal (messages + progress bars), verbose (timestamped colored logs).

**Architecture:** Refonte de `logger.rs` avec timer global et couleurs ANSI, ajout de `--no-color` flag dans CLI, mise à jour de tous les appels de logging dans `main.rs`.

**Tech Stack:** Rust, atty (TTY detection), ANSI escape codes

---

### Task 1: Add `atty` dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependency**

In `Cargo.toml`, add to `[dependencies]`:

```toml
atty = "0.2"
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add atty dependency for TTY detection"
```

---

### Task 2: Add `--no-color` flag to CLI

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add flag to Args struct**

After line 31 (`pub quiet: bool,`), add:

```rust
    /// Disable colored output (auto-detected by default).
    #[arg(long)]
    pub no_color: bool,
```

**Step 2: Add field to Config struct**

After line 100 (`pub verbosity: VerbosityLevel,`), add:

```rust
    /// Whether to disable colored output.
    pub no_color: bool,
```

**Step 3: Pass no_color in validate()**

In the `Ok(Config { ... })` block (around line 185-199), add `no_color: self.no_color,` after `verbosity,`.

**Step 4: Update test helper `create_test_args`**

In the `create_test_args` function, add `no_color: false,` to the Args struct.

**Step 5: Verify tests pass**

Run: `cargo test --lib cli`
Expected: All CLI tests pass

**Step 6: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add --no-color flag"
```

---

### Task 3: Refactor logger.rs - Add timer and color support

**Files:**
- Modify: `src/logger.rs`

**Step 1: Add imports and static timer**

Replace the imports at the top with:

```rust
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
```

**Step 2: Update Logger struct**

Replace the Logger struct with:

```rust
/// Thread-safe logger for controlling application output.
#[derive(Debug)]
pub struct Logger {
    level: VerbosityLevel,
    colors_enabled: bool,
}
```

**Step 3: Replace Logger impl**

Replace the entire `impl Logger` block with:

```rust
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
```

**Step 4: Update convenience functions**

Replace the convenience functions at the bottom with:

```rust
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
pub fn is_verbose() -> bool {
    Logger::instance().is_verbose()
}

/// Returns true if quiet mode is enabled.
#[allow(dead_code)]
pub fn is_quiet() -> bool {
    Logger::instance().is_quiet()
}
```

**Step 5: Update tests**

Replace the entire `#[cfg(test)]` module with:

```rust
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
```

**Step 6: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 7: Verify tests pass**

Run: `cargo test --lib logger`
Expected: All logger tests pass

**Step 8: Commit**

```bash
git add src/logger.rs
git commit -m "feat(logger): add timer, colors, output() and warn() functions"
```

---

### Task 4: Update main.rs - Logger init

**Files:**
- Modify: `src/main.rs`

**Step 1: Update Logger::init call**

Change line 39 from:
```rust
    logger::Logger::init(config.verbosity);
```

To:
```rust
    logger::Logger::init(config.verbosity, config.no_color);
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): pass no_color to logger init"
```

---

### Task 5: Update main.rs - Replace success with output/warn

**Files:**
- Modify: `src/main.rs`

**Step 1: Replace warning at line 70**

Change:
```rust
        eprintln!("Warning: No polygon layers found in the GeoPackage");
```

To:
```rust
        logger::warn("No polygon layers found in the GeoPackage");
```

**Step 2: Replace success at line 187**

Change:
```rust
    logger::success(&format!("Total time: {:.2?}", duration));
```

To:
```rust
    logger::info(&format!("Total time: {:.2?}", duration));
```

**Step 3: Replace success at line 295**

Change:
```rust
    logger::success(&format!("Saved: {}", output_path.display()));
```

To:
```rust
    logger::output(&output_path.display().to_string());
```

**Step 4: Replace success at line 396**

Change:
```rust
    logger::success(&format!("Total time: {:.2?}", duration));
```

To:
```rust
    logger::info(&format!("Total time: {:.2?}", duration));
```

**Step 5: Replace success at line 397**

Change:
```rust
    logger::success(&format!("Saved: {}", output_path.display()));
```

To:
```rust
    logger::output(&output_path.display().to_string());
```

**Step 6: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add src/main.rs
git commit -m "refactor(main): use output() for paths, warn() for warnings"
```

---

### Task 6: Remove duplicate debug calls

**Files:**
- Modify: `src/main.rs`

**Step 1: Remove line 313 (duplicate of 312)**

Delete this line:
```rust
    logger::debug(&format!("GeoJSON contains {} geometries", geometries.len()));
```

**Step 2: Remove lines 343-344 (duplicate of 334-338)**

Delete these lines:
```rust
    logger::debug(&format!("Resolution: {:.10} degrees/pixel", resolution));
    logger::debug(&format!("Bounding box: {:?}", bbox));
```

**Step 3: Remove line 359 (duplicate of 358)**

Delete this line:
```rust
    logger::debug(&format!("Image dimensions: {}x{} pixels", width, height));
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "refactor(main): remove duplicate debug logging"
```

---

### Task 7: Condition progress bars on verbosity

**Files:**
- Modify: `src/main.rs`

**Step 1: Update show_progress in process_gpkg (around line 146)**

Change:
```rust
    let show_progress = config.verbosity != VerbosityLevel::Quiet;
```

To:
```rust
    let show_progress = config.verbosity == VerbosityLevel::Normal;
```

**Step 2: Update show_progress in process_geojson (around line 362)**

Change:
```rust
    let show_progress = config.verbosity != VerbosityLevel::Quiet;
```

To:
```rust
    let show_progress = config.verbosity == VerbosityLevel::Normal;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "refactor(main): show progress bars only in normal mode"
```

---

### Task 8: Add per-geometry logging in verbose mode (GPKG)

**Files:**
- Modify: `src/main.rs`

**Step 1: Update geometry loop in process_layer (around lines 267-272)**

Replace:
```rust
    for (i, geom) in geometries.iter().enumerate() {
        renderer.render_multipolygon(geom);
        if let Some(ref pb) = pb {
            pb.set_position((i + 1) as u64);
        }
    }
```

With:
```rust
    let total = geometries.len();
    for (i, geom) in geometries.iter().enumerate() {
        if config.verbosity == VerbosityLevel::Verbose {
            logger::debug(&format!("Rendering geometry {}/{}", i + 1, total));
        }
        renderer.render_multipolygon(geom);
        if let Some(ref pb) = pb {
            pb.set_position((i + 1) as u64);
        }
    }
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): add per-geometry debug logging for GPKG"
```

---

### Task 9: Add per-geometry logging in verbose mode (GeoJSON)

**Files:**
- Modify: `src/main.rs`

**Step 1: Update geometry loop in process_geojson (around lines 377-382)**

Replace:
```rust
    for (i, geom) in geometries.iter().enumerate() {
        renderer.render_multipolygon(geom);
        if let Some(ref pb) = pb {
            pb.set_position((i + 1) as u64);
        }
    }
```

With:
```rust
    let total = geometries.len();
    for (i, geom) in geometries.iter().enumerate() {
        if config.verbosity == VerbosityLevel::Verbose {
            logger::debug(&format!("Rendering geometry {}/{}", i + 1, total));
        }
        renderer.render_multipolygon(geom);
        if let Some(ref pb) = pb {
            pb.set_position((i + 1) as u64);
        }
    }
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): add per-geometry debug logging for GeoJSON"
```

---

### Task 10: Remove unused logger::success function

**Files:**
- Modify: `src/logger.rs`

**Step 1: Remove success method from Logger impl**

Delete the `success` method (it's no longer used, replaced by `output`).

**Step 2: Remove success convenience function**

Delete:
```rust
/// Log a success message (displayed in quiet mode and above).
pub fn success(msg: &str) {
    Logger::instance().success(msg);
}
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/logger.rs
git commit -m "refactor(logger): remove unused success function"
```

---

### Task 11: Final verification

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 2: Test quiet mode manually**

Run: `cargo run -- test.gpkg -f gpkg -q --resolution 0.001`
Expected: Only file paths output, one per line

**Step 3: Test normal mode manually**

Run: `cargo run -- test.gpkg -f gpkg --resolution 0.001`
Expected: Progress messages + bars, no prefixes

**Step 4: Test verbose mode manually**

Run: `cargo run -- test.gpkg -f gpkg -v --resolution 0.001`
Expected: Timestamped colored logs, per-geometry detail

**Step 5: Test --no-color flag**

Run: `cargo run -- test.gpkg -f gpkg -v --no-color --resolution 0.001`
Expected: Verbose logs without colors

**Step 6: Commit any final fixes if needed**

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add atty dependency | Cargo.toml |
| 2 | Add --no-color flag | cli.rs |
| 3 | Refactor logger.rs | logger.rs |
| 4 | Update logger init | main.rs |
| 5 | Replace success with output/warn | main.rs |
| 6 | Remove duplicate debug calls | main.rs |
| 7 | Condition progress bars | main.rs |
| 8 | Per-geometry logging GPKG | main.rs |
| 9 | Per-geometry logging GeoJSON | main.rs |
| 10 | Remove unused success | logger.rs |
| 11 | Final verification | - |
