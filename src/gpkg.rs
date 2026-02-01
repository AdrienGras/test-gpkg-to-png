use geo::{Geometry, MultiPolygon};
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
}
