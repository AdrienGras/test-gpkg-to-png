# Design: gpkg-to-png CLI Tool

## Overview

A Rust CLI tool that converts polygon layers from GeoPackage files to transparent PNG images for map overlay.

## Requirements

- Input: GeoPackage (.gpkg) file with polygon/multipolygon layers
- Output: One PNG file per layer with transparency
- Rendering: Simple configurable fill/stroke colors (RGBA hex)
- Projection: Source data reprojected to WGS84 (EPSG:4326)
- Resolution: Specified in degrees per pixel

## Interface

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
  -h, --help                 Print help
  -V, --version              Print version
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

### Modules

1. **cli**: Defines `Args` struct with clap derive macros. Validates bbox format, hex colors, and resolution > 0.

2. **gpkg**:
   - List available polygon layers via `gpkg_contents` and `gpkg_geometry_columns`
   - Read geometries as WKB
   - Reproject from source CRS to WGS84 using `proj` crate
   - Return `geo::MultiPolygon<f64>` objects

3. **render**:
   - Create RGBA image buffer with calculated dimensions
   - Transform geo coordinates to pixel coordinates
   - Rasterize polygons (fill + stroke)
   - Save as PNG

4. **math**:
   - `world_to_screen()`: Convert WGS84 → pixel coordinates
   - `calculate_dimensions()`: Compute image size from bbox + resolution

## Data Flow

```
Input GPKG
    │
    ▼
┌─────────────────┐
│  List layers    │ (if --layer not specified)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Read geometries │ (as WKB)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Reproject to    │ (proj: source CRS → EPSG:4326)
│ WGS84           │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Clip to bbox    │ (AABB filter)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Render to image │ (image crate)
└────────┬────────
         │
         ▼
    Output PNG(s)
```

### Image Dimensions Calculation

```rust
width = ((max_lon - min_lon) / resolution).ceil() as u32;
height = ((max_lat - min_lat) / resolution).ceil() as u32;
```

### Coordinate Transformation

```rust
// WGS84 → pixel coordinates (Y-inverted for image coords)
x = (lon - min_lon) / resolution;
y = height as f64 - ((lat - min_lat) / resolution);
```

## Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
geo = "0.28"
image = "0.25"
proj = "0.27"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1", features = ["rt-multi-thread"] }
anyhow = "1"
thiserror = "1"
hex = "0.4"
```

## Error Handling

| Error | Behavior |
|-------|----------|
| File not found | Exit 1, clear message |
| No polygon layers | Warning, exit 0 |
| Invalid bbox | CLI validation error |
| Layer not found | Error + list available layers |
| Reprojection fails | Skip feature + warning |
| Out of memory | Pre-check dimensions, clear error |
| IO error | Context with path |

## Testing Strategy

### Unit Tests
- Argument parsing validation
- Color hex parsing
- Coordinate transformations
- Simple polygon rasterization

### Integration Tests
- Create in-memory GPKG with test data
- Verify output dimensions match expected
- Verify pixel colors at known positions

### Test Data
- `test.gpkg`: Real data (EPSG:2154, 2525 multipolygons)
- Create simplified fixture: small 1°x1° area with known shapes

## Sample Data Info

**File**: `test.gpkg` (6.1 MB)

| Property | Value |
|----------|-------|
| Layer name | `plateforme_debordement_etang_03_sans_union_20251118` |
| Geometry type | MULTIPOLYGON |
| Feature count | 2,525 |
| Source CRS | EPSG:2154 (Lambert-93) |
| Bounds (L93) | 860985,6253545 → 880674,6275762 |
| Bounds (WGS84) | ~48.4°N -4.5°W (Bretagne, France) |

Note: For git storage, consider creating a smaller fixture file (< 100 KB).
