use geo::{Geometry, MapCoords, MultiPolygon};
use proj::Proj;
use rayon::prelude::*;
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
            .map_err(GpkgError::Database)?
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
        let query = format!("SELECT {} FROM \"{}\"", layer.geometry_column, layer.name);

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

    /// Read and reproject geometries to WGS84.
    ///
    /// This method parallelizes the reprojection of geometries using `rayon`.
    /// Each thread initializes its own `proj` context for thread safety.
    pub async fn read_geometries_wgs84(&self, layer: &LayerInfo) -> Result<Vec<MultiPolygon<f64>>> {
        let geometries = self.read_geometries(layer).await?;

        if layer.srs_id == 4326 {
            return Ok(geometries);
        }

        let srs_def = self.get_srs_definition(layer.srs_id).await?;

        // Parallelize reprojection
        let reprojected: Vec<MultiPolygon<f64>> = geometries
            .into_par_iter()
            .filter_map(|mp| {
                // Proj is Send but not Sync, so we must create it per thread.
                // Using a closure with Proj inside allows each thread to have its own.
                let proj = Proj::new_known_crs(&srs_def, "EPSG:4326", None).ok()?;
                reproject_multipolygon(&mp, &proj)
            })
            .collect();

        Ok(reprojected)
    }

    /// Get the bounding box of a layer in source CRS from gpkg_contents
    pub async fn get_layer_bbox(&self, layer: &LayerInfo) -> Result<Option<(f64, f64, f64, f64)>> {
        let row = sqlx::query(
            "SELECT min_x, min_y, max_x, max_y FROM gpkg_contents WHERE table_name = ?",
        )
        .bind(&layer.name)
        .fetch_one(&self.pool)
        .await?;

        let min_x: Option<f64> = row.get("min_x");
        let min_y: Option<f64> = row.get("min_y");
        let max_x: Option<f64> = row.get("max_x");
        let max_y: Option<f64> = row.get("max_y");

        match (min_x, min_y, max_x, max_y) {
            (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) => {
                Ok(Some((min_x, min_y, max_x, max_y)))
            }
            _ => Ok(None),
        }
    }
}

/// Reproject a bbox from source CRS to WGS84.
///
/// Returns `None` if the projection fails.
/// Handles the projection by reprojecting all 4 corners and computing the new bounds.
pub fn reproject_bbox_to_wgs84(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    srs_def: &str,
) -> Option<(f64, f64, f64, f64)> {
    let proj = Proj::new_known_crs(srs_def, "EPSG:4326", None).ok()?;

    // Reproject all 4 corners and find the new bounds
    let corners = [
        (min_x, min_y),
        (max_x, min_y),
        (max_x, max_y),
        (min_x, max_y),
    ];

    let mut wgs84_min_lon = f64::MAX;
    let mut wgs84_min_lat = f64::MAX;
    let mut wgs84_max_lon = f64::MIN;
    let mut wgs84_max_lat = f64::MIN;

    for (x, y) in corners {
        if let Ok((lon, lat)) = proj.convert((x, y)) {
            wgs84_min_lon = wgs84_min_lon.min(lon);
            wgs84_min_lat = wgs84_min_lat.min(lat);
            wgs84_max_lon = wgs84_max_lon.max(lon);
            wgs84_max_lat = wgs84_max_lat.max(lat);
        }
    }

    if wgs84_min_lon == f64::MAX {
        None
    } else {
        Some((wgs84_min_lon, wgs84_min_lat, wgs84_max_lon, wgs84_max_lat))
    }
}

/// Reproject a MultiPolygon using proj.
///
/// Returns `None` if any coordinate transformation fails (results in NaN).
fn reproject_multipolygon(mp: &MultiPolygon<f64>, proj: &Proj) -> Option<MultiPolygon<f64>> {
    let reprojected = mp.map_coords(|coord| match proj.convert((coord.x, coord.y)) {
        Ok((x, y)) => geo::Coord { x, y },
        Err(_) => geo::Coord {
            x: f64::NAN,
            y: f64::NAN,
        },
    });

    // Check if any coordinates failed (became NaN)
    let has_nan = reprojected.iter().any(|poly| {
        poly.exterior()
            .coords()
            .any(|c| c.x.is_nan() || c.y.is_nan())
            || poly
                .interiors()
                .iter()
                .any(|ring| ring.coords().any(|c| c.x.is_nan() || c.y.is_nan()))
    });

    if has_nan {
        None
    } else {
        Some(reprojected)
    }
}

/// Parse GeoPackage WKB (with header) to geo Geometry.
///
/// GeoPackage WKB format extends ISO WKB with a specific header:
/// - Magic number: "GP" (2 bytes)
/// - Version: 1 byte
/// - Flags: 1 byte (includes envelope type and byte order)
/// - SRS ID: 4 bytes
/// - Optional envelope data
/// - Standard ISO WKB
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
        1 => 32, // 4 doubles (minx, maxx, miny, maxy)
        2 => 48, // 6 doubles (+ minz, maxz)
        3 => 48, // 6 doubles (+ minm, maxm)
        4 => 64, // 8 doubles (all)
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

    #[test]
    fn test_reproject_identity() {
        use geo::{coord, LineString, Polygon};

        // Test that reprojection with identity proj works
        let poly = Polygon::new(
            LineString::from(vec![
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
        if let Ok(proj) = Proj::new_known_crs("EPSG:4326", "EPSG:4326", None) {
            let result = reproject_multipolygon(&mp, &proj);
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_reproject_bbox_to_wgs84() {
        // Test reprojection from WGS84 to WGS84 (should be identity-like)
        let result = reproject_bbox_to_wgs84(-4.5, 48.0, -4.0, 48.5, "EPSG:4326");
        assert!(result.is_some());
        let (min_lon, min_lat, max_lon, max_lat) = result.unwrap();
        assert!((min_lon - (-4.5)).abs() < 0.001);
        assert!((min_lat - 48.0).abs() < 0.001);
        assert!((max_lon - (-4.0)).abs() < 0.001);
        assert!((max_lat - 48.5).abs() < 0.001);
    }

    #[test]
    fn test_reproject_bbox_from_lambert93() {
        // Test reprojection from EPSG:2154 (Lambert-93) to WGS84
        // Using approximate Lambert-93 coordinates for Brittany, France
        // These coords are approximate - main test is that reprojection works
        // and returns valid ordered bbox values
        let result = reproject_bbox_to_wgs84(860000.0, 6250000.0, 880000.0, 6280000.0, "EPSG:2154");
        assert!(result.is_some());
        let (min_lon, min_lat, max_lon, max_lat) = result.unwrap();
        // Check that we got valid values (not NaN/Inf)
        assert!(!min_lon.is_nan() && !min_lon.is_infinite());
        assert!(!min_lat.is_nan() && !min_lat.is_infinite());
        assert!(!max_lon.is_nan() && !max_lon.is_infinite());
        assert!(!max_lat.is_nan() && !max_lat.is_infinite());
        // Check ordering
        assert!(
            min_lon < max_lon,
            "min_lon {} should be < max_lon {}",
            min_lon,
            max_lon
        );
        assert!(
            min_lat < max_lat,
            "min_lat {} should be < max_lat {}",
            min_lat,
            max_lat
        );
        // Check values are in plausible range for France (roughly -10 to 10 lon, 40 to 52 lat)
        assert!(
            min_lon > -10.0 && min_lon < 10.0,
            "min_lon {} should be in France",
            min_lon
        );
        assert!(
            min_lat > 40.0 && min_lat < 52.0,
            "min_lat {} should be in France",
            min_lat
        );
    }

    #[test]
    fn test_reproject_bbox_invalid_crs() {
        // Test with invalid CRS - should return None
        let result = reproject_bbox_to_wgs84(0.0, 0.0, 1.0, 1.0, "INVALID:CRS");
        assert!(result.is_none());
    }
}
