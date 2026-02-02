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
