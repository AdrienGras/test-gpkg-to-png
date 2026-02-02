# Design: Verbosity Control (-v / -q flags)

**Date**: 2026-02-02  
**Status**: Ready for implementation

## Overview

Add `-v` (verbose) and `-q` (quiet) flags to control CLI output verbosity. Three levels:
- **Quiet**: Errors + final result only
- **Normal**: Progress bars + essential info (default)
- **Verbose**: Everything + debug details

## Architecture

### CLI Integration

```rust
// cli.rs
#[derive(Parser, Debug)]
pub struct Args {
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
    
    /// Suppress non-error output
    #[arg(short, long)]
    pub quiet: bool,
    // ... existing fields
}

#[derive(Debug, Clone, Copy)]
pub enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}

pub struct Config {
    // ... existing fields
    pub verbosity: VerbosityLevel,
}
```

Validation rules:
- `-v` and `-q` are mutually exclusive → error
- Default is `Normal` if neither flag provided

### Logger Module

New file `src/logger.rs`:

```rust
use std::sync::OnceLock;

pub struct Logger {
    level: VerbosityLevel,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

impl Logger {
    pub fn init(level: VerbosityLevel) {
        LOGGER.set(Logger { level }).unwrap();
    }
    
    pub fn instance() -> &'static Logger {
        LOGGER.get().expect("Logger not initialized")
    }
    
    pub fn error(&self, msg: &str) { /* always */ }
    pub fn success(&self, msg: &str) { /* quiet+ */ }
    pub fn info(&self, msg: &str) { /* normal+ */ }
    pub fn debug(&self, msg: &str) { /* verbose only */ }
    pub fn is_verbose(&self) -> bool { /* check level */ }
    pub fn is_quiet(&self) -> bool { /* check level */ }
}

// Convenience macros/functions
pub fn error(msg: &str) { Logger::instance().error(msg); }
pub fn success(msg: &str) { Logger::instance().success(msg); }
pub fn info(msg: &str) { Logger::instance().info(msg); }
pub fn debug(msg: &str) { Logger::instance().debug(msg); }
```

### Progress Bar Wrapper

```rust
pub struct ProgressConfig {
    pub enabled: bool,
}

impl Logger {
    pub fn progress_bar(&self, len: u64) -> Option<ProgressBar> {
        if self.level == VerbosityLevel::Quiet {
            None
        } else {
            Some(create_progress_bar(len))
        }
    }
}
```

## Data Flow

### Main Entry

1. Parse args
2. Validate (check `-v` and `-q` not both present)
3. Initialize logger with level
4. All subsequent output goes through logger

### GPKG Processing

**Quiet mode:**
- No progress bars
- No "Auto-detecting bbox"
- No per-layer timings
- Only: errors or "Saved: layer.png" at end

**Normal mode (current):**
- Keep existing progress bars
- Keep essential info messages
- Keep per-layer summary

**Verbose mode:**
- Add: config dump at start
- Add: geometry counts
- Add: detailed timings for each phase
- Add: CRS info, resolution calculations

### GeoJSON Processing

Same pattern as GPKG but single output file.

## Implementation Plan

1. **Create logger module** (`src/logger.rs`)
   - Define `VerbosityLevel` enum
   - Implement `Logger` struct with `OnceLock`
   - Add output methods

2. **Update CLI** (`src/cli.rs`)
   - Add `-v` and `-q` flags to `Args`
   - Add validation for mutual exclusivity
   - Add `verbosity` field to `Config`
   - Update all test helpers

3. **Update main** (`src/main.rs`)
   - Initialize logger with config verbosity
   - Replace `println!` with logger calls
   - Wrap progress bars with verbosity check
   - Add verbose-only debug output

4. **Tests**
   - Unit tests for logger at each level
   - CLI validation tests for `-v` + `-q`
   - Integration tests verifying output levels

## Behavior Matrix

| Output Type | Quiet | Normal | Verbose |
|-------------|-------|--------|---------|
| Errors | ✓ | ✓ | ✓ |
| "Saved: file.png" | ✓ | ✓ | ✓ |
| Progress bars | ✗ | ✓ | ✓ |
| "Processing..." info | ✗ | ✓ | ✓ |
| Debug details | ✗ | ✗ | ✓ |
| Config dump | ✗ | ✗ | ✓ |
| Geometry counts | ✗ | ✗ | ✓ |
| Per-phase timings | ✗ | ✗ | ✓ |

## Error Handling

- Mutual exclusivity error: clear message directing user to choose one
- Logger init failure: panic (should never happen - init once at startup)
- All existing error handling unchanged

## Testing Strategy

1. **Unit tests** (`logger.rs`)
   - Test each level filters correctly
   - Test `is_verbose()`, `is_quiet()` helpers

2. **CLI tests** (`cli.rs`)
   - Test `-v` alone → Verbose
   - Test `-q` alone → Quiet
   - Test both together → error
   - Test neither → Normal

3. **Integration tests**
   - Capture stdout/stderr and verify content per level
   - Verify quiet mode produces minimal output
   - Verify verbose mode includes debug info

## Migration Guide

Replace existing outputs:

```rust
// Before
println!("Auto-detecting bbox...");

// After  
logger::info("Auto-detecting bbox...");

// Before
println!("Total time: {:.2?}", duration);

// After
logger::success(&format!("Total time: {:.2?}", duration));

// Debug-only
if verbose {
    logger::debug(&format!("Read {} geometries", count));
}
```
