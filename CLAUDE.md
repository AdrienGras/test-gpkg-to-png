# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Rust CLI tool that converts polygon layers from GeoPackage (GPKG) files to transparent PNG images for map overlay.

## CLI Interface

```bash
gpkg-to-png <input> [OPTIONS]

Arguments:
  <input>  Path to the .gpkg file

Options:
  -o, --output-dir <DIR>     Output directory [default: .]
  -b, --bbox <BBOX>          Bounding box: "minLon,minLat,maxLon,maxLat" [required]
  -r, --resolution <RES>     Pixel size in degrees [required]
      --fill <COLOR>         Fill color RGBA hex [default: "FF000080"]
      --stroke <COLOR>       Stroke color RGB hex [default: "FF0000"]
      --stroke-width <WIDTH> Stroke width in pixels [default: 1]
  -l, --layer <NAME>         Specific layer to render (default: all)
```

### Example

```bash
gpkg-to-png test.gpkg \
  --bbox "-4.5,48.0,-4.0,48.5" \
  --resolution 0.0001 \
  --fill "00FF0080" \
  --stroke "00FF00" \
  --stroke-width 2 \
  -o ./output/
```

## Architecture

```
src/
├── main.rs       # Entry point, error handling
├── cli.rs        # Argument parsing with clap
├── gpkg.rs       # GeoPackage reading with sqlx
├── render.rs     # Raster rendering with image/geo
├── math.rs       # Coordinate transformations
└── error.rs      # Error types
```

### Module Responsibilities

- **cli**: Parses and validates arguments (bbox format, hex colors, resolution)
- **gpkg**: Lists layers, reads WKB geometries, reprojects to WGS84 using `proj`
- **render**: Creates RGBA buffer, rasterizes polygons, saves PNG
- **math**: World-to-screen coordinate conversion, dimension calculations

### Data Flow

1. Parse CLI arguments
2. List available polygon layers from `gpkg_contents`/`gpkg_geometry_columns`
3. For each layer:
   - Read geometries as WKB
   - Reproject from source CRS to EPSG:4326
   - Clip to bbox
   - Rasterize (fill + stroke)
   - Save as PNG

## Key Dependencies

- `clap` - CLI argument parsing
- `geo` - Geometric types and operations
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
