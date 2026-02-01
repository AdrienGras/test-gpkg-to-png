# gpkg-to-png Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Créer un outil CLI Rust qui convertit les couches polygones d'un fichier GeoPackage en images PNG transparentes pour superposition cartographique.

**Architecture:** Structure modulaire avec séparation des responsabilités : CLI (parsing), GPKG (lecture), Render (rastérisation), Math (transformations). Approche TDD avec tests unitaires et d'intégration.

**Tech Stack:** Rust, clap (CLI), geo (géométrie), image (raster), proj (reprojection), sqlx (SQLite/GPKG)

---

## Task 1: Initialisation du projet Cargo

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Step 1: Créer le projet Cargo**

```bash
cargo init --name gpkg-to-png
```

**Step 2: Configurer Cargo.toml avec les dépendances**

```toml
[package]
name = "gpkg-to-png"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
geo = "0.28"
image = "0.25"
proj = "0.27"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
anyhow = "1"
thiserror = "1"
hex = "0.4"
wkb = "0.7"

[dev-dependencies]
tempfile = "3"
```

**Step 3: Vérifier que le projet compile**

Run: `cargo build`
Expected: Compilation réussie

**Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "chore: initialize cargo project with dependencies"
```

---

## Task 2: Module error - Types d'erreurs

**Files:**
- Create: `src/error.rs`
- Modify: `src/main.rs`

**Step 1: Écrire le test pour les types d'erreur**

Create `src/error.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpkgError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("No polygon layers found in the GeoPackage")]
    NoPolygonLayers,

    #[error("Layer '{0}' not found. Available layers: {1}")]
    LayerNotFound(String, String),

    #[error("Invalid bounding box format: {0}")]
    InvalidBbox(String),

    #[error("Invalid color format: {0}")]
    InvalidColor(String),

    #[error("Resolution must be positive, got: {0}")]
    InvalidResolution(f64),

    #[error("Image dimensions too large: {width}x{height} pixels (max: {max})")]
    ImageTooLarge { width: u32, height: u32, max: u32 },

    #[error("Reprojection failed for feature: {0}")]
    ReprojectionFailed(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, GpkgError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GpkgError::FileNotFound("test.gpkg".to_string());
        assert_eq!(err.to_string(), "File not found: test.gpkg");
    }

    #[test]
    fn test_layer_not_found_display() {
        let err = GpkgError::LayerNotFound("foo".to_string(), "bar, baz".to_string());
        assert_eq!(err.to_string(), "Layer 'foo' not found. Available layers: bar, baz");
    }
}
```

**Step 2: Mettre à jour main.rs pour déclarer le module**

```rust
mod error;

fn main() {
    println!("gpkg-to-png");
}
```

**Step 3: Exécuter les tests**

Run: `cargo test error`
Expected: 2 tests passent

**Step 4: Commit**

```bash
git add src/error.rs src/main.rs
git commit -m "feat(error): add error types with thiserror"
```

---

## Task 3: Module math - Transformations de coordonnées

**Files:**
- Create: `src/math.rs`
- Modify: `src/main.rs`

**Step 1: Écrire les tests pour calculate_dimensions**

Create `src/math.rs`:

```rust
/// Bounding box in WGS84 coordinates
#[derive(Debug, Clone, Copy)]
pub struct Bbox {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

impl Bbox {
    pub fn new(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Self {
        Self { min_lon, min_lat, max_lon, max_lat }
    }

    pub fn width(&self) -> f64 {
        self.max_lon - self.min_lon
    }

    pub fn height(&self) -> f64 {
        self.max_lat - self.min_lat
    }
}

/// Calculate image dimensions from bbox and resolution
pub fn calculate_dimensions(bbox: &Bbox, resolution: f64) -> (u32, u32) {
    let width = (bbox.width() / resolution).ceil() as u32;
    let height = (bbox.height() / resolution).ceil() as u32;
    (width, height)
}

/// Convert WGS84 coordinates to pixel coordinates
/// Y is inverted for image coordinate system (0,0 at top-left)
pub fn world_to_screen(lon: f64, lat: f64, bbox: &Bbox, resolution: f64, height: u32) -> (f64, f64) {
    let x = (lon - bbox.min_lon) / resolution;
    let y = height as f64 - ((lat - bbox.min_lat) / resolution);
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbox_dimensions() {
        let bbox = Bbox::new(-4.5, 48.0, -4.0, 48.5);
        assert!((bbox.width() - 0.5).abs() < 1e-10);
        assert!((bbox.height() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_dimensions() {
        let bbox = Bbox::new(-4.5, 48.0, -4.0, 48.5);
        let resolution = 0.001;
        let (width, height) = calculate_dimensions(&bbox, resolution);
        assert_eq!(width, 500);
        assert_eq!(height, 500);
    }

    #[test]
    fn test_calculate_dimensions_rounds_up() {
        let bbox = Bbox::new(0.0, 0.0, 1.0, 1.0);
        let resolution = 0.3; // 1.0 / 0.3 = 3.333...
        let (width, height) = calculate_dimensions(&bbox, resolution);
        assert_eq!(width, 4);
        assert_eq!(height, 4);
    }

    #[test]
    fn test_world_to_screen_origin() {
        let bbox = Bbox::new(0.0, 0.0, 1.0, 1.0);
        let resolution = 0.1;
        let (_, height) = calculate_dimensions(&bbox, resolution);

        // Bottom-left corner of bbox -> bottom-left of image (0, height)
        let (x, y) = world_to_screen(0.0, 0.0, &bbox, resolution, height);
        assert!((x - 0.0).abs() < 1e-10);
        assert!((y - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_world_to_screen_top_right() {
        let bbox = Bbox::new(0.0, 0.0, 1.0, 1.0);
        let resolution = 0.1;
        let (width, height) = calculate_dimensions(&bbox, resolution);

        // Top-right corner of bbox -> top-right of image (width, 0)
        let (x, y) = world_to_screen(1.0, 1.0, &bbox, resolution, height);
        assert!((x - width as f64).abs() < 1e-10);
        assert!((y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_world_to_screen_center() {
        let bbox = Bbox::new(0.0, 0.0, 1.0, 1.0);
        let resolution = 0.1;
        let (_, height) = calculate_dimensions(&bbox, resolution);

        let (x, y) = world_to_screen(0.5, 0.5, &bbox, resolution, height);
        assert!((x - 5.0).abs() < 1e-10);
        assert!((y - 5.0).abs() < 1e-10);
    }
}
```

**Step 2: Mettre à jour main.rs**

```rust
mod error;
mod math;

fn main() {
    println!("gpkg-to-png");
}
```

**Step 3: Exécuter les tests**

Run: `cargo test math`
Expected: 6 tests passent

**Step 4: Commit**

```bash
git add src/math.rs src/main.rs
git commit -m "feat(math): add coordinate transformation functions"
```

---

## Task 4: Module cli - Parsing des arguments

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Écrire le module cli avec tests**

Create `src/cli.rs`:

```rust
use clap::Parser;
use std::path::PathBuf;

use crate::error::{GpkgError, Result};
use crate::math::Bbox;

/// Convert GeoPackage polygon layers to PNG images
#[derive(Parser, Debug)]
#[command(name = "gpkg-to-png")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the .gpkg file
    pub input: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,

    /// Bounding box: "minLon,minLat,maxLon,maxLat"
    #[arg(short, long)]
    pub bbox: String,

    /// Pixel size in degrees
    #[arg(short, long)]
    pub resolution: f64,

    /// Fill color RGBA hex (e.g., "FF000080")
    #[arg(long, default_value = "FF000080")]
    pub fill: String,

    /// Stroke color RGB hex (e.g., "FF0000")
    #[arg(long, default_value = "FF0000")]
    pub stroke: String,

    /// Stroke width in pixels
    #[arg(long, default_value = "1")]
    pub stroke_width: u32,

    /// Specific layer to render (default: all)
    #[arg(short, long)]
    pub layer: Option<String>,
}

/// Parsed and validated configuration
#[derive(Debug)]
pub struct Config {
    pub input: PathBuf,
    pub output_dir: PathBuf,
    pub bbox: Bbox,
    pub resolution: f64,
    pub fill: [u8; 4],
    pub stroke: [u8; 3],
    pub stroke_width: u32,
    pub layer: Option<String>,
}

impl Args {
    pub fn validate(self) -> Result<Config> {
        // Validate resolution
        if self.resolution <= 0.0 {
            return Err(GpkgError::InvalidResolution(self.resolution));
        }

        // Parse bbox
        let bbox = parse_bbox(&self.bbox)?;

        // Parse colors
        let fill = parse_rgba(&self.fill)?;
        let stroke = parse_rgb(&self.stroke)?;

        Ok(Config {
            input: self.input,
            output_dir: self.output_dir,
            bbox,
            resolution: self.resolution,
            fill,
            stroke,
            stroke_width: self.stroke_width,
            layer: self.layer,
        })
    }
}

fn parse_bbox(s: &str) -> Result<Bbox> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(GpkgError::InvalidBbox(format!(
            "expected 4 comma-separated values, got {}",
            parts.len()
        )));
    }

    let values: std::result::Result<Vec<f64>, _> = parts.iter().map(|p| p.trim().parse()).collect();
    let values = values.map_err(|_| GpkgError::InvalidBbox("invalid number format".to_string()))?;

    Ok(Bbox::new(values[0], values[1], values[2], values[3]))
}

fn parse_rgba(s: &str) -> Result<[u8; 4]> {
    let bytes = hex::decode(s).map_err(|_| GpkgError::InvalidColor(s.to_string()))?;
    if bytes.len() != 4 {
        return Err(GpkgError::InvalidColor(format!(
            "RGBA color must be 8 hex digits, got {}",
            s.len()
        )));
    }
    Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn parse_rgb(s: &str) -> Result<[u8; 3]> {
    let bytes = hex::decode(s).map_err(|_| GpkgError::InvalidColor(s.to_string()))?;
    if bytes.len() != 3 {
        return Err(GpkgError::InvalidColor(format!(
            "RGB color must be 6 hex digits, got {}",
            s.len()
        )));
    }
    Ok([bytes[0], bytes[1], bytes[2]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bbox_valid() {
        let bbox = parse_bbox("-4.5,48.0,-4.0,48.5").unwrap();
        assert!((bbox.min_lon - (-4.5)).abs() < 1e-10);
        assert!((bbox.min_lat - 48.0).abs() < 1e-10);
        assert!((bbox.max_lon - (-4.0)).abs() < 1e-10);
        assert!((bbox.max_lat - 48.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_bbox_with_spaces() {
        let bbox = parse_bbox(" -4.5 , 48.0 , -4.0 , 48.5 ").unwrap();
        assert!((bbox.min_lon - (-4.5)).abs() < 1e-10);
    }

    #[test]
    fn test_parse_bbox_invalid_count() {
        let err = parse_bbox("-4.5,48.0,-4.0").unwrap_err();
        assert!(err.to_string().contains("expected 4"));
    }

    #[test]
    fn test_parse_bbox_invalid_number() {
        let err = parse_bbox("-4.5,abc,-4.0,48.5").unwrap_err();
        assert!(err.to_string().contains("invalid number"));
    }

    #[test]
    fn test_parse_rgba_valid() {
        let color = parse_rgba("FF000080").unwrap();
        assert_eq!(color, [255, 0, 0, 128]);
    }

    #[test]
    fn test_parse_rgba_invalid_length() {
        let err = parse_rgba("FF0000").unwrap_err();
        assert!(err.to_string().contains("8 hex digits"));
    }

    #[test]
    fn test_parse_rgba_invalid_hex() {
        let err = parse_rgba("GGGGGGGG").unwrap_err();
        assert!(err.to_string().contains("Invalid color"));
    }

    #[test]
    fn test_parse_rgb_valid() {
        let color = parse_rgb("00FF00").unwrap();
        assert_eq!(color, [0, 255, 0]);
    }

    #[test]
    fn test_parse_rgb_invalid_length() {
        let err = parse_rgb("FF").unwrap_err();
        assert!(err.to_string().contains("6 hex digits"));
    }
}
```

**Step 2: Mettre à jour main.rs**

```rust
mod cli;
mod error;
mod math;

use clap::Parser;
use cli::Args;

fn main() {
    let args = Args::parse();
    match args.validate() {
        Ok(config) => {
            println!("Input: {:?}", config.input);
            println!("Bbox: {:?}", config.bbox);
            println!("Resolution: {}", config.resolution);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Step 3: Exécuter les tests**

Run: `cargo test cli`
Expected: 9 tests passent

**Step 4: Tester le CLI**

Run: `cargo run -- --help`
Expected: Affiche l'aide avec toutes les options

**Step 5: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat(cli): add argument parsing with validation"
```

---

## Task 5: Module gpkg - Lecture des couches

**Files:**
- Create: `src/gpkg.rs`
- Modify: `src/main.rs`

**Step 1: Créer le module gpkg avec les structures de base**

Create `src/gpkg.rs`:

```rust
use geo::{Geometry, MultiPolygon, Polygon};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;

use crate::error::{GpkgError, Result};

/// Information about a polygon layer in the GeoPackage
#[derive(Debug, Clone)]
pub struct LayerInfo {
    pub name: String,
    pub geometry_column: String,
    pub srs_id: i32,
}

/// Read GeoPackage and extract polygon layers
pub struct GpkgReader {
    pool: SqlitePool,
}

impl GpkgReader {
    /// Open a GeoPackage file
    pub async fn open(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(GpkgError::FileNotFound(path.display().to_string()));
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))
            .map_err(|e| GpkgError::Database(e))?
            .read_only(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(GpkgError::Database)?;

        Ok(Self { pool })
    }

    /// List all polygon/multipolygon layers
    pub async fn list_polygon_layers(&self) -> Result<Vec<LayerInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT c.table_name, g.column_name, g.srs_id
            FROM gpkg_contents c
            JOIN gpkg_geometry_columns g ON c.table_name = g.table_name
            WHERE c.data_type = 'features'
            AND (g.geometry_type_name LIKE '%POLYGON%' OR g.geometry_type_name LIKE '%polygon%')
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let layers: Vec<LayerInfo> = rows
            .iter()
            .map(|row| LayerInfo {
                name: row.get("table_name"),
                geometry_column: row.get("column_name"),
                srs_id: row.get("srs_id"),
            })
            .collect();

        Ok(layers)
    }

    /// Read geometries from a specific layer
    pub async fn read_geometries(&self, layer: &LayerInfo) -> Result<Vec<MultiPolygon<f64>>> {
        let query = format!(
            "SELECT {} FROM \"{}\"",
            layer.geometry_column, layer.name
        );

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let mut geometries = Vec::new();
        for row in rows {
            let wkb_data: Vec<u8> = row.get(0);

            // Skip GeoPackage header (first 8 bytes: magic, version, flags, srs_id, envelope)
            // GeoPackage WKB has a header before the standard WKB
            if let Some(geom) = parse_gpkg_wkb(&wkb_data) {
                match geom {
                    Geometry::Polygon(p) => geometries.push(MultiPolygon::new(vec![p])),
                    Geometry::MultiPolygon(mp) => geometries.push(mp),
                    _ => {} // Skip non-polygon geometries
                }
            }
        }

        Ok(geometries)
    }

    /// Get SRS definition for a layer
    pub async fn get_srs_definition(&self, srs_id: i32) -> Result<String> {
        let row = sqlx::query("SELECT definition FROM gpkg_spatial_ref_sys WHERE srs_id = ?")
            .bind(srs_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("definition"))
    }
}

/// Parse GeoPackage WKB (with header) to geo Geometry
fn parse_gpkg_wkb(data: &[u8]) -> Option<Geometry<f64>> {
    // GeoPackage uses a header before standard WKB
    // Header: magic (2 bytes), version (1 byte), flags (1 byte), srs_id (4 bytes)
    // Then optional envelope, then standard WKB

    if data.len() < 8 {
        return None;
    }

    // Check magic number "GP"
    if data[0] != 0x47 || data[1] != 0x50 {
        // Try parsing as standard WKB
        return wkb::wkb_to_geom(&mut std::io::Cursor::new(data)).ok();
    }

    let flags = data[3];
    let envelope_indicator = (flags >> 1) & 0x07;

    // Calculate envelope size based on indicator
    let envelope_size = match envelope_indicator {
        0 => 0,
        1 => 32,  // 4 doubles (minx, maxx, miny, maxy)
        2 => 48,  // 6 doubles (+ minz, maxz)
        3 => 48,  // 6 doubles (+ minm, maxm)
        4 => 64,  // 8 doubles (all)
        _ => return None,
    };

    let wkb_start = 8 + envelope_size;
    if data.len() <= wkb_start {
        return None;
    }

    wkb::wkb_to_geom(&mut std::io::Cursor::new(&data[wkb_start..])).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests with real GPKG file in tests/ directory

    #[test]
    fn test_layer_info_creation() {
        let layer = LayerInfo {
            name: "test_layer".to_string(),
            geometry_column: "geom".to_string(),
            srs_id: 4326,
        };
        assert_eq!(layer.name, "test_layer");
        assert_eq!(layer.srs_id, 4326);
    }
}
```

**Step 2: Mettre à jour main.rs**

```rust
mod cli;
mod error;
mod gpkg;
mod math;

use clap::Parser;
use cli::Args;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.validate() {
        Ok(config) => {
            println!("Input: {:?}", config.input);
            println!("Bbox: {:?}", config.bbox);
            println!("Resolution: {}", config.resolution);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Step 3: Exécuter les tests**

Run: `cargo test gpkg`
Expected: Tests passent

**Step 4: Commit**

```bash
git add src/gpkg.rs src/main.rs
git commit -m "feat(gpkg): add GeoPackage reader for polygon layers"
```

---

## Task 6: Module gpkg - Reprojection vers WGS84

**Files:**
- Modify: `src/gpkg.rs`

**Step 1: Ajouter la reprojection avec proj**

Modifier `src/gpkg.rs` - ajouter après la fonction `read_geometries`:

```rust
use geo::MapCoords;
use proj::Proj;

impl GpkgReader {
    // ... existing methods ...

    /// Read and reproject geometries to WGS84
    pub async fn read_geometries_wgs84(&self, layer: &LayerInfo) -> Result<Vec<MultiPolygon<f64>>> {
        let geometries = self.read_geometries(layer).await?;

        if layer.srs_id == 4326 {
            return Ok(geometries);
        }

        let srs_def = self.get_srs_definition(layer.srs_id).await?;

        // Create projection from source CRS to WGS84
        let proj = Proj::new_known_crs(&srs_def, "EPSG:4326", None)
            .ok_or_else(|| GpkgError::ReprojectionFailed(format!(
                "Could not create projection from SRS {} to WGS84", layer.srs_id
            )))?;

        let reprojected: Vec<MultiPolygon<f64>> = geometries
            .into_iter()
            .filter_map(|mp| reproject_multipolygon(&mp, &proj))
            .collect();

        Ok(reprojected)
    }
}

/// Reproject a MultiPolygon using proj
fn reproject_multipolygon(mp: &MultiPolygon<f64>, proj: &Proj) -> Option<MultiPolygon<f64>> {
    let reprojected = mp.map_coords(|coord| {
        match proj.convert((coord.x, coord.y)) {
            Ok((x, y)) => geo::Coord { x, y },
            Err(_) => geo::Coord { x: f64::NAN, y: f64::NAN },
        }
    });

    // Check if any coordinates failed (became NaN)
    let has_nan = reprojected.iter().any(|poly| {
        poly.exterior().coords().any(|c| c.x.is_nan() || c.y.is_nan())
    });

    if has_nan {
        None
    } else {
        Some(reprojected)
    }
}
```

**Step 2: Ajouter le test unitaire**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use geo::coord;

    // ... existing tests ...

    #[test]
    fn test_reproject_identity() {
        // Test that reprojection with identity proj works
        // This is a simplified test - real integration tests use actual proj
        let poly = Polygon::new(
            geo::LineString::from(vec![
                coord! { x: 0.0, y: 0.0 },
                coord! { x: 1.0, y: 0.0 },
                coord! { x: 1.0, y: 1.0 },
                coord! { x: 0.0, y: 1.0 },
                coord! { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let mp = MultiPolygon::new(vec![poly]);

        // Create an identity-like projection (WGS84 to WGS84)
        if let Some(proj) = Proj::new_known_crs("EPSG:4326", "EPSG:4326", None) {
            let result = reproject_multipolygon(&mp, &proj);
            assert!(result.is_some());
        }
    }
}
```

**Step 3: Exécuter les tests**

Run: `cargo test gpkg`
Expected: Tests passent

**Step 4: Commit**

```bash
git add src/gpkg.rs
git commit -m "feat(gpkg): add reprojection to WGS84 using proj"
```

---

## Task 7: Module render - Création d'image et rastérisation

**Files:**
- Create: `src/render.rs`
- Modify: `src/main.rs`

**Step 1: Créer le module render avec tests**

Create `src/render.rs`:

```rust
use geo::{BoundingRect, Coord, MultiPolygon};
use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

use crate::error::{GpkgError, Result};
use crate::math::{calculate_dimensions, world_to_screen, Bbox};

const MAX_DIMENSION: u32 = 20000;

/// Render configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub bbox: Bbox,
    pub resolution: f64,
    pub fill: [u8; 4],
    pub stroke: [u8; 3],
    pub stroke_width: u32,
}

/// Renderer for MultiPolygon geometries to PNG
pub struct Renderer {
    config: RenderConfig,
    width: u32,
    height: u32,
    image: RgbaImage,
}

impl Renderer {
    /// Create a new renderer with the given configuration
    pub fn new(config: RenderConfig) -> Result<Self> {
        let (width, height) = calculate_dimensions(&config.bbox, config.resolution);

        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(GpkgError::ImageTooLarge {
                width,
                height,
                max: MAX_DIMENSION,
            });
        }

        let image = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));

        Ok(Self {
            config,
            width,
            height,
            image,
        })
    }

    /// Get image dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Render a multipolygon onto the image
    pub fn render_multipolygon(&mut self, mp: &MultiPolygon<f64>) {
        for polygon in mp.iter() {
            self.render_polygon_fill(polygon);
        }

        if self.config.stroke_width > 0 {
            for polygon in mp.iter() {
                self.render_polygon_stroke(polygon);
            }
        }
    }

    /// Fill a polygon
    fn render_polygon_fill(&mut self, polygon: &geo::Polygon<f64>) {
        // Get bounding box of polygon for optimization
        let Some(rect) = polygon.bounding_rect() else {
            return;
        };

        // Convert to screen coordinates
        let (min_x, max_y) = world_to_screen(
            rect.min().x,
            rect.min().y,
            &self.config.bbox,
            self.config.resolution,
            self.height,
        );
        let (max_x, min_y) = world_to_screen(
            rect.max().x,
            rect.max().y,
            &self.config.bbox,
            self.config.resolution,
            self.height,
        );

        // Clamp to image bounds
        let start_x = (min_x.floor() as i32).max(0) as u32;
        let end_x = (max_x.ceil() as i32).min(self.width as i32) as u32;
        let start_y = (min_y.floor() as i32).max(0) as u32;
        let end_y = (max_y.ceil() as i32).min(self.height as i32) as u32;

        let fill = Rgba(self.config.fill);

        // Scanline fill
        for py in start_y..end_y {
            for px in start_x..end_x {
                let world = self.screen_to_world(px, py);
                if point_in_polygon(world, polygon) {
                    blend_pixel(&mut self.image, px, py, fill);
                }
            }
        }
    }

    /// Draw polygon stroke
    fn render_polygon_stroke(&mut self, polygon: &geo::Polygon<f64>) {
        let stroke = Rgba([
            self.config.stroke[0],
            self.config.stroke[1],
            self.config.stroke[2],
            255,
        ]);
        let width = self.config.stroke_width;

        // Draw exterior ring
        self.draw_linestring(polygon.exterior().coords(), stroke, width);

        // Draw interior rings (holes)
        for interior in polygon.interiors() {
            self.draw_linestring(interior.coords(), stroke, width);
        }
    }

    /// Draw a linestring with given color and width
    fn draw_linestring<'a>(
        &mut self,
        coords: impl Iterator<Item = Coord<f64>>,
        color: Rgba<u8>,
        width: u32,
    ) {
        let screen_coords: Vec<(f64, f64)> = coords
            .map(|c| {
                world_to_screen(
                    c.x,
                    c.y,
                    &self.config.bbox,
                    self.config.resolution,
                    self.height,
                )
            })
            .collect();

        for window in screen_coords.windows(2) {
            self.draw_line(window[0], window[1], color, width);
        }
    }

    /// Draw a line segment using Bresenham's algorithm
    fn draw_line(&mut self, from: (f64, f64), to: (f64, f64), color: Rgba<u8>, width: u32) {
        let (x0, y0) = (from.0 as i32, from.1 as i32);
        let (x1, y1) = (to.0 as i32, to.1 as i32);

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        let half_width = (width / 2) as i32;

        loop {
            // Draw thick line by drawing a square at each point
            for wx in -half_width..=half_width {
                for wy in -half_width..=half_width {
                    let px = x + wx;
                    let py = y + wy;
                    if px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32 {
                        blend_pixel(&mut self.image, px as u32, py as u32, color);
                    }
                }
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Convert screen coordinates back to world coordinates
    fn screen_to_world(&self, px: u32, py: u32) -> Coord<f64> {
        let lon = self.config.bbox.min_lon + (px as f64 + 0.5) * self.config.resolution;
        let lat = self.config.bbox.max_lat - (py as f64 + 0.5) * self.config.resolution;
        Coord { x: lon, y: lat }
    }

    /// Save the image to a PNG file
    pub fn save(&self, path: &Path) -> Result<()> {
        self.image.save(path)?;
        Ok(())
    }
}

/// Check if a point is inside a polygon using ray casting
fn point_in_polygon(point: Coord<f64>, polygon: &geo::Polygon<f64>) -> bool {
    let mut inside = point_in_ring(point, polygon.exterior());

    // Check holes - if inside a hole, we're outside the polygon
    for interior in polygon.interiors() {
        if point_in_ring(point, interior) {
            inside = !inside;
        }
    }

    inside
}

/// Ray casting algorithm for a single ring
fn point_in_ring(point: Coord<f64>, ring: &geo::LineString<f64>) -> bool {
    let coords: Vec<_> = ring.coords().collect();
    let n = coords.len();
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let xi = coords[i].x;
        let yi = coords[i].y;
        let xj = coords[j].x;
        let yj = coords[j].y;

        if ((yi > point.y) != (yj > point.y))
            && (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi)
        {
            inside = !inside;
        }

        j = i;
    }

    inside
}

/// Blend a pixel with alpha compositing
fn blend_pixel(image: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>) {
    let dst = image.get_pixel(x, y);
    let src_a = color.0[3] as f32 / 255.0;
    let dst_a = dst.0[3] as f32 / 255.0;

    let out_a = src_a + dst_a * (1.0 - src_a);

    if out_a == 0.0 {
        return;
    }

    let blend = |src: u8, dst: u8| -> u8 {
        let src = src as f32;
        let dst = dst as f32;
        ((src * src_a + dst * dst_a * (1.0 - src_a)) / out_a) as u8
    };

    image.put_pixel(
        x,
        y,
        Rgba([
            blend(color.0[0], dst.0[0]),
            blend(color.0[1], dst.0[1]),
            blend(color.0[2], dst.0[2]),
            (out_a * 255.0) as u8,
        ]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{coord, LineString, Polygon};

    #[test]
    fn test_renderer_creation() {
        let config = RenderConfig {
            bbox: Bbox::new(0.0, 0.0, 1.0, 1.0),
            resolution: 0.01,
            fill: [255, 0, 0, 128],
            stroke: [255, 0, 0],
            stroke_width: 1,
        };
        let renderer = Renderer::new(config).unwrap();
        assert_eq!(renderer.dimensions(), (100, 100));
    }

    #[test]
    fn test_renderer_too_large() {
        let config = RenderConfig {
            bbox: Bbox::new(0.0, 0.0, 100.0, 100.0),
            resolution: 0.0001, // Would create 1M x 1M image
            fill: [255, 0, 0, 128],
            stroke: [255, 0, 0],
            stroke_width: 1,
        };
        let result = Renderer::new(config);
        assert!(matches!(result, Err(GpkgError::ImageTooLarge { .. })));
    }

    #[test]
    fn test_point_in_polygon() {
        let polygon = Polygon::new(
            LineString::from(vec![
                coord! { x: 0.0, y: 0.0 },
                coord! { x: 10.0, y: 0.0 },
                coord! { x: 10.0, y: 10.0 },
                coord! { x: 0.0, y: 10.0 },
                coord! { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );

        assert!(point_in_polygon(coord! { x: 5.0, y: 5.0 }, &polygon));
        assert!(!point_in_polygon(coord! { x: -1.0, y: 5.0 }, &polygon));
        assert!(!point_in_polygon(coord! { x: 15.0, y: 5.0 }, &polygon));
    }

    #[test]
    fn test_point_in_polygon_with_hole() {
        let exterior = LineString::from(vec![
            coord! { x: 0.0, y: 0.0 },
            coord! { x: 10.0, y: 0.0 },
            coord! { x: 10.0, y: 10.0 },
            coord! { x: 0.0, y: 10.0 },
            coord! { x: 0.0, y: 0.0 },
        ]);
        let hole = LineString::from(vec![
            coord! { x: 3.0, y: 3.0 },
            coord! { x: 7.0, y: 3.0 },
            coord! { x: 7.0, y: 7.0 },
            coord! { x: 3.0, y: 7.0 },
            coord! { x: 3.0, y: 3.0 },
        ]);
        let polygon = Polygon::new(exterior, vec![hole]);

        // Inside the polygon but outside the hole
        assert!(point_in_polygon(coord! { x: 1.0, y: 1.0 }, &polygon));
        // Inside the hole
        assert!(!point_in_polygon(coord! { x: 5.0, y: 5.0 }, &polygon));
    }

    #[test]
    fn test_render_simple_polygon() {
        let config = RenderConfig {
            bbox: Bbox::new(0.0, 0.0, 10.0, 10.0),
            resolution: 1.0,
            fill: [255, 0, 0, 255],
            stroke: [0, 0, 0],
            stroke_width: 0,
        };
        let mut renderer = Renderer::new(config).unwrap();

        let polygon = Polygon::new(
            LineString::from(vec![
                coord! { x: 2.0, y: 2.0 },
                coord! { x: 8.0, y: 2.0 },
                coord! { x: 8.0, y: 8.0 },
                coord! { x: 2.0, y: 8.0 },
                coord! { x: 2.0, y: 2.0 },
            ]),
            vec![],
        );
        let mp = MultiPolygon::new(vec![polygon]);

        renderer.render_multipolygon(&mp);

        // Check center pixel is filled
        let center = renderer.image.get_pixel(5, 5);
        assert_eq!(center.0, [255, 0, 0, 255]);

        // Check corner pixel is transparent
        let corner = renderer.image.get_pixel(0, 0);
        assert_eq!(corner.0, [0, 0, 0, 0]);
    }
}
```

**Step 2: Mettre à jour main.rs**

```rust
mod cli;
mod error;
mod gpkg;
mod math;
mod render;

use clap::Parser;
use cli::Args;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.validate() {
        Ok(config) => {
            println!("Input: {:?}", config.input);
            println!("Bbox: {:?}", config.bbox);
            println!("Resolution: {}", config.resolution);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Step 3: Exécuter les tests**

Run: `cargo test render`
Expected: 5 tests passent

**Step 4: Commit**

```bash
git add src/render.rs src/main.rs
git commit -m "feat(render): add polygon rasterization with fill and stroke"
```

---

## Task 8: Intégration main.rs - Pipeline complet

**Files:**
- Modify: `src/main.rs`

**Step 1: Implémenter le pipeline complet**

Remplacer le contenu de `src/main.rs`:

```rust
mod cli;
mod error;
mod gpkg;
mod math;
mod render;

use clap::Parser;
use std::path::Path;

use cli::Args;
use error::{GpkgError, Result};
use gpkg::GpkgReader;
use render::{RenderConfig, Renderer};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();
    let config = args.validate()?;

    // Check input file exists
    if !config.input.exists() {
        return Err(GpkgError::FileNotFound(config.input.display().to_string()));
    }

    // Create output directory if needed
    if !config.output_dir.exists() {
        std::fs::create_dir_all(&config.output_dir)?;
    }

    // Open GeoPackage
    let reader = GpkgReader::open(&config.input).await?;

    // Get layers to process
    let all_layers = reader.list_polygon_layers().await?;

    if all_layers.is_empty() {
        eprintln!("Warning: No polygon layers found in the GeoPackage");
        return Ok(());
    }

    let layers_to_process = match &config.layer {
        Some(name) => {
            let layer = all_layers
                .iter()
                .find(|l| l.name == *name)
                .ok_or_else(|| {
                    let available = all_layers
                        .iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    GpkgError::LayerNotFound(name.clone(), available)
                })?;
            vec![layer.clone()]
        }
        None => all_layers,
    };

    println!("Processing {} layer(s)...", layers_to_process.len());

    // Process each layer
    for layer in &layers_to_process {
        println!("  Layer: {}", layer.name);

        // Read and reproject geometries
        let geometries = reader.read_geometries_wgs84(&layer).await?;
        println!("    {} geometries", geometries.len());

        if geometries.is_empty() {
            println!("    Skipping: no geometries");
            continue;
        }

        // Create renderer
        let render_config = RenderConfig {
            bbox: config.bbox,
            resolution: config.resolution,
            fill: config.fill,
            stroke: config.stroke,
            stroke_width: config.stroke_width,
        };

        let mut renderer = Renderer::new(render_config)?;
        let (width, height) = renderer.dimensions();
        println!("    Image: {}x{} pixels", width, height);

        // Render all geometries
        for geom in &geometries {
            renderer.render_multipolygon(geom);
        }

        // Save output
        let output_path = config.output_dir.join(format!("{}.png", layer.name));
        renderer.save(&output_path)?;
        println!("    Saved: {}", output_path.display());
    }

    println!("Done!");
    Ok(())
}
```

**Step 2: Vérifier que tout compile**

Run: `cargo build`
Expected: Compilation réussie

**Step 3: Tester avec le fichier test.gpkg**

Run: `cargo run -- test.gpkg --bbox "-4.8,48.2,-4.3,48.6" --resolution 0.0005 -o ./output/`
Expected: Génère un fichier PNG dans ./output/

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: integrate full pipeline in main"
```

---

## Task 9: Tests d'intégration

**Files:**
- Create: `tests/integration.rs`

**Step 1: Créer les tests d'intégration**

Create `tests/integration.rs`:

```rust
use std::path::Path;
use std::process::Command;

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("gpkg-to-png"));
    assert!(stdout.contains("--bbox"));
    assert!(stdout.contains("--resolution"));
}

#[test]
fn test_missing_required_args() {
    let output = Command::new("cargo")
        .args(["run", "--", "test.gpkg"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--bbox") || stderr.contains("required"));
}

#[test]
fn test_invalid_bbox() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.gpkg",
            "--bbox",
            "invalid",
            "--resolution",
            "0.001",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("bbox") || stderr.contains("expected 4"));
}

#[test]
fn test_file_not_found() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "nonexistent.gpkg",
            "--bbox",
            "-4.5,48.0,-4.0,48.5",
            "--resolution",
            "0.001",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("File"));
}

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_real_gpkg_file() {
    // This test requires test.gpkg to be present
    if !Path::new("test.gpkg").exists() {
        eprintln!("Skipping: test.gpkg not found");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "test.gpkg",
            "--bbox",
            "-4.8,48.2,-4.3,48.6",
            "--resolution",
            "0.001",
            "-o",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    assert!(output.status.success());

    // Check that at least one PNG was created
    let png_files: Vec<_> = std::fs::read_dir(temp_dir.path())
        .expect("Failed to read temp dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "png"))
        .collect();

    assert!(!png_files.is_empty(), "No PNG files were created");
}
```

**Step 2: Exécuter les tests d'intégration de base**

Run: `cargo test --test integration`
Expected: Tests passent (sauf le test ignoré)

**Step 3: Exécuter le test complet avec test.gpkg**

Run: `cargo test --test integration -- --ignored`
Expected: Test passe si test.gpkg existe

**Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for CLI"
```

---

## Task 10: Documentation et finalisation

**Files:**
- Modify: `CLAUDE.md` (si nécessaire)
- Create: `README.md` (optionnel, seulement si demandé)

**Step 1: Vérifier que tous les tests passent**

Run: `cargo test`
Expected: Tous les tests passent

**Step 2: Vérifier le formatage**

Run: `cargo fmt --check`
Expected: Pas de changements nécessaires

**Step 3: Vérifier avec clippy**

Run: `cargo clippy -- -D warnings`
Expected: Pas d'avertissements

**Step 4: Commit final**

```bash
git add -A
git commit -m "chore: finalize project setup and documentation"
```

---

## Résumé des fichiers

| Fichier | Description |
|---------|-------------|
| `Cargo.toml` | Configuration du projet et dépendances |
| `src/main.rs` | Point d'entrée, pipeline principal |
| `src/cli.rs` | Parsing des arguments CLI avec clap |
| `src/error.rs` | Types d'erreurs avec thiserror |
| `src/math.rs` | Transformations de coordonnées |
| `src/gpkg.rs` | Lecture GeoPackage et reprojection |
| `src/render.rs` | Rastérisation des polygones |
| `tests/integration.rs` | Tests d'intégration CLI |

## Commandes de test

```bash
# Tous les tests unitaires
cargo test

# Tests d'intégration uniquement
cargo test --test integration

# Test avec fichier réel
cargo test --test integration -- --ignored

# Test manuel
cargo run -- test.gpkg --bbox "-4.8,48.2,-4.3,48.6" --resolution 0.0005 -o ./output/
```
