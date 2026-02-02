# GeoJSON Input Support Design

**Date:** 2026-02-02
**Status:** Approved
**Author:** Claude with Adrien

## Overview

Add support for GeoJSON files as input format alongside the existing GeoPackage support. Users will be able to render GeoJSON files to PNG using the same rendering pipeline.

## Goals

- Support GeoJSON files as an alternative to GPKG input
- Handle both raw geometries and FeatureCollection formats
- Maintain the existing GPKG workflow unchanged
- Minimize code duplication and impact on existing modules

## Non-Goals

- Layer support for GeoJSON (one file = one PNG output)
- Custom CRS support (always assume WGS84/EPSG:4326)
- Support for non-polygon geometries (Point, LineString ignored)

## Architecture

### CLI Changes (src/cli.rs)

Add two new fields to `Args`:

```rust
/// Input file format
#[arg(short = 'f', long, value_enum)]
pub format: Format,

/// Output PNG filename (GeoJSON only, default: input filename)
#[arg(long)]
pub output_name: Option<String>,
```

Add new enum:

```rust
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Format {
    Gpkg,
    Geojson,
}
```

**Validation rules:**
- If `format = geojson` and `--layer` is specified → return error (incompatible)
- Default output name for GeoJSON = input filename without extension

### New Module: src/geojson.rs

```rust
pub struct GeojsonReader {
    geometries: Vec<geo::MultiPolygon<f64>>,
}

impl GeojsonReader {
    /// Opens and parses a GeoJSON file
    pub async fn open(path: &Path) -> Result<Self>

    /// Returns all parsed geometries
    pub fn get_geometries(&self) -> &[geo::MultiPolygon<f64>]

    /// Computes the bounding box from all geometries
    pub fn compute_bbox(&self) -> Option<Bbox>
}
```

**Parsing logic:**

1. Read JSON file using `serde_json`
2. Detect type:
   - `"type": "FeatureCollection"` → extract geometries from each feature
   - `"type": "MultiPolygon"` or `"Polygon"` → raw geometry
   - `"type": "GeometryCollection"` → extract geometries
3. Convert to `geo::MultiPolygon<f64>`:
   - Polygon → wrap in a MultiPolygon with 1 polygon
   - MultiPolygon → direct conversion
   - Ignore Point, LineString, etc.

**CRS assumption:**
- Always assume WGS84 (EPSG:4326) per GeoJSON RFC 7946
- No reprojection needed

### Changes to main.rs

Refactor `run()` to dispatch based on format:

```rust
async fn run() -> Result<()> {
    let args = Args::parse();
    let config = args.validate()?;

    // File existence check
    // Create output directory

    match config.format {
        Format::Gpkg => process_gpkg(config).await?,
        Format::Geojson => process_geojson(config).await?,
    }

    Ok(())
}
```

**New function: `process_geojson()`**

1. Open GeojsonReader
2. Get geometries
3. Determine bbox (from CLI or auto-detect from geometries)
4. Calculate resolution (from --resolution or --scale)
5. Create Renderer with config
6. Render all geometries
7. Save PNG:
   - If `--output-name` provided → use that
   - Otherwise → input filename without extension

**New function: `process_gpkg()`**

Move existing `run()` logic here (multi-layer GPKG workflow unchanged).

## Data Flow

### GeoJSON Path

```
GeoJSON file
  ↓
GeojsonReader::open()
  ↓ (parse JSON, convert to geo types)
Vec<MultiPolygon<f64>>
  ↓
Compute/validate bbox
  ↓
Renderer (existing)
  ↓
PNG output
```

### GPKG Path (unchanged)

```
GPKG file
  ↓
GpkgReader::open()
  ↓
List layers
  ↓
For each layer:
  Read WKB → Reproject → Vec<MultiPolygon<f64>>
  ↓
  Renderer
  ↓
  PNG per layer
```

## Dependencies

Add to Cargo.toml:

```toml
geojson = "0.24"  # Official GeoJSON parser with geo support
```

## Error Handling

Add new error variants to `GpkgError` enum (consider renaming to `AppError`):

- `GeojsonParseError(String)` - JSON parsing failed
- `UnsupportedGeometryType(String)` - non-polygon geometry encountered
- `EmptyGeojson` - no polygon geometries found in file
- `InvalidFormatOption(String)` - incompatible CLI options (e.g., --layer with geojson)

## Testing Strategy

### Unit Tests (src/geojson.rs)

- Parse FeatureCollection with multiple features
- Parse raw MultiPolygon geometry
- Parse raw Polygon (convert to MultiPolygon)
- Ignore Point and LineString geometries
- Handle empty or invalid JSON
- Compute bbox correctly from geometries

### Integration Tests

Using test.geojson and test2.geojson:

- Render test.geojson to PNG
- Render test2.geojson to PNG
- Verify output dimensions match expected bbox
- Test custom --output-name
- Verify pixel content (sample point checks)

### CLI Tests

- Error when --layer specified with geojson format
- Error when invalid format specified
- Default output name uses input filename

## Implementation Checklist

- [ ] Add `geojson` dependency to Cargo.toml
- [ ] Create src/geojson.rs with GeojsonReader
- [ ] Add Format enum and CLI fields to src/cli.rs
- [ ] Update validation logic in src/cli.rs
- [ ] Add new error variants to src/error.rs
- [ ] Refactor main.rs: extract process_gpkg() and add process_geojson()
- [ ] Write unit tests for geojson.rs
- [ ] Write integration tests with test.geojson files
- [ ] Update CLAUDE.md and README.md with new CLI options
- [ ] Manual testing with both test files

## Future Enhancements (Out of Scope)

- Support for other formats (Shapefile, KML, etc.)
- Custom CRS support via --input-crs flag
- Layer extraction from GeoJSON properties
- Geometry type conversion (LineString to Polygon buffer)
