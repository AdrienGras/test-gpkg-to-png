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

## Architecture

```
src/
├── main.rs       # Entry point, tokio async pipeline, format dispatch
├── cli.rs        # Argument parsing with clap, color/bbox validation
├── gpkg.rs       # GeoPackage reading with sqlx, PROJ reprojection
├── geojson.rs    # GeoJSON reading and parsing (assumes WGS84)
├── render.rs     # Main rendering orchestration with image/geo
├── render/
│   └── edge.rs   # Edge table handling for scanline rasterization
├── math.rs       # Coordinate transformations
└── error.rs      # Error types
```

### Module Responsibilities

- **cli**: Parses and validates arguments (bbox format, hex colors, resolution/scale)
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

---

*Last updated: 2026-02-02 after GeoJSON support merge*
