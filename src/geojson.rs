//! GeoJSON file reading and parsing.

use geo::{Coord, LineString, MultiPolygon, Polygon};
use geojson::{GeoJson, Geometry, Value};
use std::fs;
use std::path::Path;

use crate::error::{GpkgError, Result};
use crate::math::Bbox;

/// Reader for GeoJSON files.
///
/// Parses GeoJSON and extracts polygon geometries.
/// Assumes WGS84 (EPSG:4326) coordinate reference system.
pub struct GeojsonReader {
    geometries: Vec<MultiPolygon<f64>>,
}

impl GeojsonReader {
    /// Opens and parses a GeoJSON file.
    pub async fn open(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GpkgError::FileNotFound(path.display().to_string())
            } else {
                GpkgError::Io(e)
            }
        })?;

        let content = preprocess_geojson(&content);

        let geojson: GeoJson = content.parse().map_err(|e| {
            GpkgError::GeojsonParseError(format!("{}", e))
        })?;

        let geometries = extract_geometries(&geojson);

        if geometries.is_empty() {
            return Err(GpkgError::EmptyGeojson);
        }

        Ok(Self { geometries })
    }

    /// Returns all parsed geometries.
    pub fn get_geometries(&self) -> &[MultiPolygon<f64>] {
        &self.geometries
    }

    /// Computes the bounding box from all geometries.
    pub fn compute_bbox(&self) -> Option<Bbox> {
        if self.geometries.is_empty() {
            return None;
        }

        let mut min_lon = f64::MAX;
        let mut min_lat = f64::MAX;
        let mut max_lon = f64::MIN;
        let mut max_lat = f64::MIN;

        for mp in &self.geometries {
            for poly in mp.iter() {
                for coord in poly.exterior().coords() {
                    min_lon = min_lon.min(coord.x);
                    min_lat = min_lat.min(coord.y);
                    max_lon = max_lon.max(coord.x);
                    max_lat = max_lat.max(coord.y);
                }
                for interior in poly.interiors() {
                    for coord in interior.coords() {
                        min_lon = min_lon.min(coord.x);
                        min_lat = min_lat.min(coord.y);
                        max_lon = max_lon.max(coord.x);
                        max_lat = max_lat.max(coord.y);
                    }
                }
            }
        }

        if min_lon == f64::MAX {
            None
        } else {
            Some(Bbox::new(min_lon, min_lat, max_lon, max_lat))
        }
    }
}

/// Pre-process GeoJSON content to fix common malformed patterns.
fn preprocess_geojson(content: &str) -> String {
    // Fix empty type field ("type":"" -> "type":"MultiPolygon")
    // Handles malformed GeoJSON where type field is empty
    // Must be done BEFORE CSV fix to avoid breaking the empty string
    let content = content.replace(r#""type":"""#, r#""type":"MultiPolygon""#);
    let content = content.replace(r#""type": """#, r#""type": "MultiPolygon""#);

    // Fix CSV-style double-quote escaping (""type"" -> "type")
    // This handles malformed GeoJSON exported from some tools
    content.replace("\"\"", "\"")
}

/// Extract polygon geometries from GeoJSON.
fn extract_geometries(geojson: &GeoJson) -> Vec<MultiPolygon<f64>> {
    let mut geometries = Vec::new();

    match geojson {
        GeoJson::Geometry(geom) => {
            if let Some(mp) = geometry_to_multipolygon(geom) {
                geometries.push(mp);
            }
        }
        GeoJson::Feature(feature) => {
            if let Some(ref geom) = feature.geometry {
                if let Some(mp) = geometry_to_multipolygon(geom) {
                    geometries.push(mp);
                }
            }
        }
        GeoJson::FeatureCollection(collection) => {
            for feature in &collection.features {
                if let Some(ref geom) = feature.geometry {
                    if let Some(mp) = geometry_to_multipolygon(geom) {
                        geometries.push(mp);
                    }
                }
            }
        }
    }

    geometries
}

/// Convert a GeoJSON geometry to a MultiPolygon.
fn geometry_to_multipolygon(geom: &Geometry) -> Option<MultiPolygon<f64>> {
    match &geom.value {
        Value::Polygon(coords) => {
            let polygon = polygon_from_coords(coords)?;
            Some(MultiPolygon::new(vec![polygon]))
        }
        Value::MultiPolygon(multi_coords) => {
            let polygons: Vec<Polygon<f64>> = multi_coords
                .iter()
                .filter_map(|coords| polygon_from_coords(coords))
                .collect();
            if polygons.is_empty() {
                None
            } else {
                Some(MultiPolygon::new(polygons))
            }
        }
        _ => None, // Ignore other geometry types
    }
}

/// Convert GeoJSON polygon coordinates to geo Polygon.
fn polygon_from_coords(coords: &[Vec<Vec<f64>>]) -> Option<Polygon<f64>> {
    if coords.is_empty() {
        return None;
    }

    let exterior = linestring_from_coords(&coords[0])?;
    let interiors: Vec<LineString<f64>> = coords[1..]
        .iter()
        .filter_map(|ring| linestring_from_coords(ring))
        .collect();

    Some(Polygon::new(exterior, interiors))
}

/// Convert GeoJSON ring coordinates to geo LineString.
fn linestring_from_coords(coords: &[Vec<f64>]) -> Option<LineString<f64>> {
    if coords.is_empty() {
        return None;
    }

    let points: Vec<Coord<f64>> = coords
        .iter()
        .filter_map(|point| {
            if point.len() >= 2 {
                Some(Coord {
                    x: point[0],
                    y: point[1],
                })
            } else {
                None
            }
        })
        .collect();

    if points.is_empty() {
        None
    } else {
        Some(LineString::from(points))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feature_collection() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
                    }
                }
            ]
        }"#;

        let geojson: GeoJson = json.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 1);
    }

    #[test]
    fn test_parse_raw_polygon() {
        let json = r#"{
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
        }"#;

        let geojson: GeoJson = json.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 1);
    }

    #[test]
    fn test_parse_raw_multipolygon() {
        let json = r#"{
            "type": "MultiPolygon",
            "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
        }"#;

        let geojson: GeoJson = json.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 1);
    }

    #[test]
    fn test_parse_empty_type_root_geometry() {
        let json = r#"{
            "type": "",
            "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
        }"#;

        let processed = preprocess_geojson(json);
        let geojson: GeoJson = processed.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 1);
    }

    #[test]
    fn test_parse_empty_type_in_feature() {
        let json = r#"{
            "type": "Feature",
            "geometry": {
                "type": "",
                "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
            }
        }"#;

        let processed = preprocess_geojson(json);
        let geojson: GeoJson = processed.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 1);
    }

    #[test]
    fn test_parse_empty_type_in_featurecollection() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "",
                        "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]]
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[2.0, 2.0], [3.0, 2.0], [3.0, 3.0], [2.0, 3.0], [2.0, 2.0]]]
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "",
                        "coordinates": [[[[4.0, 4.0], [5.0, 4.0], [5.0, 5.0], [4.0, 5.0], [4.0, 4.0]]]]
                    }
                }
            ]
        }"#;

        let processed = preprocess_geojson(json);
        let geojson: GeoJson = processed.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 3); // All three features should be parsed
    }

    #[test]
    fn test_ignore_non_polygon_geometries() {
        let json = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Point",
                        "coordinates": [0.0, 0.0]
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
                    }
                }
            ]
        }"#;

        let geojson: GeoJson = json.parse().unwrap();
        let geometries = extract_geometries(&geojson);
        assert_eq!(geometries.len(), 1);
    }

    #[test]
    fn test_compute_bbox_from_geometries() {
        let poly1 = Polygon::new(
            LineString::from(vec![
                Coord { x: 0.0, y: 0.0 },
                Coord { x: 1.0, y: 0.0 },
                Coord { x: 1.0, y: 1.0 },
                Coord { x: 0.0, y: 1.0 },
                Coord { x: 0.0, y: 0.0 },
            ]),
            vec![],
        );
        let poly2 = Polygon::new(
            LineString::from(vec![
                Coord { x: 2.0, y: 2.0 },
                Coord { x: 3.0, y: 2.0 },
                Coord { x: 3.0, y: 3.0 },
                Coord { x: 2.0, y: 3.0 },
                Coord { x: 2.0, y: 2.0 },
            ]),
            vec![],
        );

        let reader = GeojsonReader {
            geometries: vec![
                MultiPolygon::new(vec![poly1]),
                MultiPolygon::new(vec![poly2]),
            ],
        };

        let bbox = reader.compute_bbox().unwrap();
        assert!((bbox.min_lon - 0.0).abs() < 1e-10);
        assert!((bbox.min_lat - 0.0).abs() < 1e-10);
        assert!((bbox.max_lon - 3.0).abs() < 1e-10);
        assert!((bbox.max_lat - 3.0).abs() < 1e-10);
    }
}
