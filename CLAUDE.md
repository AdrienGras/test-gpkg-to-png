# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust CLI tool that converts polygon layers from GeoPackage (GPKG) files to transparent PNG images for map overlay.

## CLI Interface

### Basic Commands
```bash
cargo build --release  # Build the project
cargo test             # Run unit tests
cargo test --test integration -- --ignored # Run integration tests (req. test data)
```

```bash
gpkg-to-png <input> [OPTIONS]

Arguments:
  <input>  Path to the input file (.gpkg or .geojson)

Options:
  -f, --format <FORMAT>      Input format: gpkg or geojson [required]
  -v, --verbose              Verbose mode: timestamped colored logs, per-geometry detail
  -q, --quiet                Quiet mode: output only file paths (one per line)
      --no-color             Disable ANSI colors (auto-detected if not TTY)
  -o, --output-dir <DIR>     Output directory [default: .]
  -b, --bbox <BBOX>          Bounding box: "minLon,minLat,maxLon,maxLat" [auto-detected if omitted]
  -r, --resolution <RES>     Pixel size in degrees (mutually exclusive with --scale)
  -s, --scale <SCALE>        Meters per pixel (mutually exclusive with --resolution)
      --fill <COLOR>         Fill color RGBA hex [default: "FF000080"]
      --stroke <COLOR>       Stroke color RGB hex [default: "FF0000"]
      --stroke-width <WIDTH> Stroke width in pixels [default: 1]
  -l, --layer <NAME>         Specific layer to render (GPKG only, default: all)
      --output-name <NAME>   Output PNG filename (GeoJSON only, default: input filename)
```

### Examples

**GeoPackage:**
```bash
gpkg-to-png test.gpkg \
  -f gpkg \
  --bbox "-4.5,48.0,-4.0,48.5" \
  --resolution 0.0001 \
  --fill "00FF0080" \
  --stroke "00FF00" \
  --stroke-width 2 \
  -o ./output/
```

**GeoJSON:**
```bash
gpkg-to-png test.geojson \
  -f geojson \
  --bbox "5.166,43.381,5.168,43.383" \
  --resolution 0.00001 \
  --output-name "my-map" \
  -o ./output/
```

**Verbose mode (with timestamps and colors):**
```bash
gpkg-to-png test.gpkg -f gpkg -v --resolution 0.0001
# [0.00s] [INFO] Auto-detecting bounding box...
# [0.02s] [INFO] Processing 1 layer(s)...
# [0.03s] [DEBUG] Rendering geometry 1/100
# ...
# [1.23s] [INFO] Saved: ./layer.png
```

**Quiet mode (for scripting):**
```bash
gpkg-to-png test.gpkg -f gpkg -q --resolution 0.0001
# ./layer.png
```

## Architecture

```
src/
├── main.rs       # Entry point, tokio async pipeline, format dispatch
├── cli.rs        # Argument parsing with clap, color/bbox validation
├── logger.rs     # Verbosity control: quiet/normal/verbose modes, colors, timestamps
├── gpkg.rs       # GeoPackage reading with sqlx, PROJ reprojection
├── geojson.rs    # GeoJSON reading and parsing (assumes WGS84)
├── render.rs     # Main rendering orchestration with image/geo
├── render/
│   └── edge.rs   # Edge table handling for scanline rasterization
├── math.rs       # Coordinate transformations
└── error.rs      # Error types
```

### Module Responsibilities

- **cli**: Parses and validates arguments (bbox format, hex colors, resolution/scale, verbosity)
- **logger**: Global logger with three modes (quiet/normal/verbose), ANSI colors, elapsed timestamps
- **gpkg**: Lists layers, reads WKB geometries, reprojects to WGS84 using `proj`
- **geojson**: Reads and parses GeoJSON files, extracts polygon geometries, assumes WGS84
- **render**: Parallel rendering (rayon), uses scanline algorithm for fills
- **math**: World-to-screen coordinate conversion, dimension calculations

### Key Technologies

- `rayon` for massive parallelism during rendering
- `tokio` and `sqlx` for asynchronous database access

### Data Flow

**GeoPackage (GPKG):**
1. Parse CLI arguments
2. List available polygon layers from `gpkg_contents`/`gpkg_geometry_columns`
3. For each layer:
   - Read geometries as WKB
   - Reproject from source CRS to EPSG:4326
   - Clip to bbox
   - Rasterize (fill + stroke)
   - Save as PNG

**GeoJSON:**
1. Parse CLI arguments
2. Read and parse GeoJSON file
3. Extract polygon/multipolygon geometries (ignore other types)
4. Auto-detect or use provided bbox
5. Rasterize (fill + stroke)
6. Save as single PNG

## Key Dependencies

- `clap` - CLI argument parsing
- `geo` - Geometric types and operations
- `geojson` - GeoJSON parsing and conversion to geo types
- `image` - Raster image creation
- `proj` - CRS reprojection
- `sqlx` - SQLite/GeoPackage access
- `atty` - TTY detection for automatic color support

## Test Data

**File**: `test.gpkg` (6.1 MB)

| Property | Value |
|----------|-------|
| Layer | `plateforme_debordement_etang_03_sans_union_20251118` |
| Type | MULTIPOLYGON |
| Count | 2,525 features |
| Source CRS | EPSG:2154 (Lambert-93) |
| Bounds (L93) | 860985,6253545 → 880674,6275762 |
| Location | Bretagne, France (~48.4°N, -4.5°W) |

## Design Document

See `docs/plans/2026-02-01-gpkg-to-png-design.md` for the full design specification.

## Lessons Learned

### Feature: GeoJSON Input Support (2026-02-02)

#### ✅ What Worked Well

**1. Subagent-Driven Development Workflow**
- 10 tasks executed with fresh subagent per task + two-stage review (spec compliance → code quality)
- Zero regressions: All 48 unit tests passed throughout
- Clean separation: Each subagent focused on single task without context pollution
- Review loops caught issues early (e.g., missing GPKG+output-name validation in Task 3)

**2. Test-Driven Development**
- Writing tests first revealed bugs before they reached production
- Integration tests (Task 8) uncovered 2 critical bugs:
  - CSV-style quote escaping in GeoJSON parser (`""` → `"`)
  - Out-of-bounds error in scanline renderer edge case
- 48 unit tests + 3 integration tests = 100% spec coverage

**3. Git Worktrees for Isolation**
- Clean separation between main and feature development
- No context switching during implementation
- Easy cleanup after merge

**4. Comprehensive Documentation**
- Design doc upfront prevented scope creep
- CLAUDE.md kept in sync with implementation
- README updated to reflect new capabilities

#### 🔧 What Could Be Improved

**1. Async/Await Usage**
- `GeojsonReader::open()` marked `async` but uses synchronous `fs::read_to_string()`
- Should either use `tokio::fs::read_to_string().await` or remove `async` keyword
- Not blocking but inconsistent with async design pattern

**2. DRY Opportunity**
- Resolution calculation from scale duplicated in `process_gpkg()` and `process_geojson()`
- Could extract to shared helper function: `compute_resolution(scale, bbox)`

**3. Large File Handling**
- GeoJSON loaded entirely into memory
- No file size validation or streaming parser
- Acceptable for typical use cases but could be enhanced for very large files

#### 🎯 Best Practices Identified

**1. Format-Specific Validation Symmetry**
- GPKG cannot use `--output-name` ✓
- GeoJSON cannot use `--layer` ✓
- Symmetric validation improves UX consistency

**2. Two-Stage Code Review**
- Stage 1: Spec compliance (does it match requirements?)
- Stage 2: Code quality (is it well-built?)
- Catching spec issues before quality review saves iteration cycles

**3. Pragmatic Error Handling**
- CSV quote escaping workaround for real-world malformed data
- Clear error messages guide users to fix issues
- Edge case handling prevents panics (bounds checking in renderer)

**4. Minimal Impact Principle**
- GPKG module completely untouched (zero regression risk)
- Render module only 2 lines changed (bounds fix)
- New functionality isolated in new modules

#### ⚠️ Pitfalls Avoided

**1. Over-Engineering**
- Didn't create abstract trait for readers (GeometrySource) - kept it simple
- Added features only when needed, not for "future flexibility"
- YAGNI principle strictly followed

**2. Breaking Changes**
- All existing GPKG tests updated to use `-f gpkg` flag
- CLI maintains backward compatibility through clear error messages
- No silent behavior changes

**3. Incomplete Testing**
- Integration tests discovered bugs unit tests missed
- Manual testing verified end-to-end workflows
- Error handling paths explicitly tested

#### 📋 Recommendations for Future Features

**1. Always Use Worktrees**
- Create isolated workspace for each feature
- Prevents accidental commits to main
- Easy cleanup after merge

**2. Write Design Doc First**
- Invest 15-20 minutes in design upfront
- Prevents scope creep and miscommunication
- Serves as spec for code review

**3. Follow TDD Rigorously**
- Write tests before implementation
- Run tests to verify they fail
- Implement minimal code to pass
- Refactor with confidence

**4. Leverage Subagent-Driven Development**
- Break work into 10-minute tasks
- Fresh subagent per task prevents context pollution
- Two-stage review catches issues early

**5. Manual Testing Is Critical**
- Unit tests don't catch integration issues
- Real-world data reveals edge cases (CSV escaping, out-of-bounds)
- Always test with actual data files

**6. Document Lessons Immediately**
- Capture learnings while fresh
- Update this section after each major feature
- Share patterns and anti-patterns

#### 🔢 Metrics (GeoJSON Feature)

- **Implementation Time**: ~90 minutes (10 tasks × ~9 min/task)
- **Code Quality**: Zero clippy errors, 1 minor warning (pre-existing)
- **Test Coverage**: 48 unit tests + 3 integration tests, all passing
- **Bug Discovery**: 2 bugs found during integration testing (both fixed)
- **Documentation**: Design doc, CLAUDE.md, README all updated
- **Commits**: 10 clean, atomic commits with conventional commit messages

#### 💡 Key Insight

**"Tests are cheaper than debugging"** - The 2 bugs found during integration testing would have cost hours to debug in production. Investing 15 minutes in integration tests saved significant future pain.

### Feature: Empty Type MultiPolygon Support (2026-02-02)

#### ✅ What Worked Well

**1. Following Established Patterns**
- String replacement approach consistent with CSV fix (line 32)
- Minimal code change (3 lines + tests)
- Zero risk of regression

**2. Test-Driven Development**
- Three tests covering all use cases (root, Feature, FeatureCollection)
- Tests written before implementation confirmed
- All tests pass on first run after implementation

**3. Simple Solution Over Complex**
- Rejected parsing with serde_json (too complex)
- Rejected regex approach (unnecessary)
- String replacement sufficient and maintainable

#### 🎯 Best Practices Identified

**1. Pattern Consistency**
- New malformed data fixes should follow established patterns
- Comment style matches existing code
- Placement logical (with other pre-processing)

**2. Comprehensive Test Coverage**
- Test root geometry, Feature, and FeatureCollection
- Test mixed valid/invalid types
- Verify all existing tests still pass

#### 💡 Key Insight

**"Follow the grain of the codebase"** - When a pattern exists for similar problems (CSV escaping), extend it rather than invent new approaches. Consistency beats novelty.

### Feature: Verbosity System Refactor (2026-02-02)

#### 📋 Context

The initial verbosity implementation (by another LLM) had issues:
- Quiet mode showed too much output (timings, messages)
- Normal mode had logging prefixes that weren't wanted
- Verbose mode lacked timestamps and colors
- No TTY detection for automatic color support

**Goal:** Three distinct, well-defined modes with clear separation.

#### ✅ What Worked Well

**1. Brainstorming Skill for Requirements Gathering**
- Interactive Q&A clarified exact requirements before coding
- Multiple-choice questions reduced ambiguity
- User specified exact output format (e.g., `[0.00s] [INFO]` with separate brackets)
- Design document written collaboratively prevented misunderstandings

**2. Subagent-Driven Development at Scale**
- 11 tasks executed with fresh subagent per task
- Spec compliance review after each task caught issues early
- Zero regressions: All 60+ tests passed throughout
- Clean atomic commits (12 total)

**3. Incremental Refactoring Strategy**
- Task ordering ensured code always compiled:
  1. Add dependency (atty)
  2. Add CLI flag (--no-color)
  3. Refactor logger (keep success temporarily)
  4. Update call sites
  5. Remove unused code (success)
- Each commit was independently functional

**4. Worktree Isolation**
- Feature branch `feature/verbosity-refactor` in `.worktrees/`
- Main branch untouched during development
- Fast-forward merge at the end
- Clean worktree removal after merge

**5. Comprehensive Final Verification**
- All three modes tested manually with real data
- `--no-color` flag tested explicitly
- TTY auto-detection verified (colors disabled in non-TTY)

#### 🔧 Implementation Details

**Logger Architecture:**
```rust
static LOGGER: OnceLock<Logger> = OnceLock::new();
static START_TIME: OnceLock<Instant> = OnceLock::new();

struct Logger {
    level: VerbosityLevel,
    colors_enabled: bool,
}
```

**Color Detection Logic:**
```rust
let colors_enabled = !no_color
    && std::env::var("NO_COLOR").is_err()
    && atty::is(atty::Stream::Stdout);
```

**Output Behavior Matrix:**

| Function | Quiet | Normal | Verbose |
|----------|-------|--------|---------|
| `output(path)` | `path` | `Saved: path` | `[0.00s] [INFO] Saved: path` |
| `info(msg)` | (silent) | `msg` | `[0.00s] [INFO] msg` |
| `debug(msg)` | (silent) | (silent) | `[0.00s] [DEBUG] msg` |
| `warn(msg)` | (silent) | `msg` | `[0.00s] [WARN] msg` |
| `error(msg)` | `Error: msg` | `Error: msg` | `[0.00s] [ERROR] msg` |
| Progress bars | No | Yes | No (replaced by debug logs) |

**ANSI Color Codes:**
- ERROR: `\x1b[31m` (red)
- WARN: `\x1b[33m` (yellow)
- INFO: `\x1b[34m` (blue)
- DEBUG: `\x1b[90m` (gray)
- Timestamp: `\x1b[90m` (gray)

#### 🎯 Best Practices Identified

**1. Design Before Code**
- Brainstorming session produced complete spec
- Output format examples in design doc matched final implementation
- No scope creep during implementation

**2. Preserving Backward Compatibility**
- Kept `success()` function until all call sites migrated
- Removed in dedicated cleanup task
- Gradual migration prevents compilation breaks

**3. Duplicate Removal as Separate Task**
- Identified 4 duplicate debug calls during audit
- Removed in dedicated task (not mixed with feature work)
- Cleaner git history

**4. Progress Bars vs Verbose Logging**
- Progress bars in Normal mode (user feedback)
- Debug logs in Verbose mode (developer debugging)
- Mutually exclusive: `show_progress = config.verbosity == VerbosityLevel::Normal`

**5. Per-Geometry Logging**
- Added `Rendering geometry X/Y` in verbose mode
- Useful for debugging large datasets
- Conditional: only when `VerbosityLevel::Verbose`

#### ⚠️ Pitfalls Avoided

**1. Global Mutable State**
- Used `OnceLock` for thread-safe singleton
- Logger initialized once at startup
- No runtime mutex contention

**2. Over-Engineering Colors**
- Simple ANSI escape codes inline
- No color library dependency needed
- `atty` (30KB) vs `colored` (larger, more features)

**3. Breaking Existing Tests**
- All existing tests continued to pass
- `#[allow(dead_code)]` for unused helper functions (is_verbose)
- Clean compiler output (zero warnings)

#### 🔢 Metrics

- **Tasks**: 11 (+ 1 warning fix)
- **Commits**: 12 atomic commits
- **Files Changed**: 4 (cli.rs, logger.rs, main.rs, Cargo.toml)
- **Lines Added**: ~150
- **Lines Removed**: ~70
- **Tests**: 60 unit + 5 integration, all passing
- **Dependencies Added**: 1 (atty)

#### 💡 Key Insights

**"Brainstorming before coding saves rework"** - The interactive Q&A session produced a complete spec with exact output formats. Implementation matched spec 100% with no rework.

**"Refactoring is ordering"** - The task order (add dep → add flag → refactor → migrate → cleanup) ensured the code always compiled. Each step was independently releasable.

**"TTY detection is essential for CLI tools"** - Automatic color disabling when piped to files or CI prevents ANSI garbage in logs. The `NO_COLOR` env var standard is also respected.

---

*Last updated: 2026-02-02 after verbosity system refactor*
