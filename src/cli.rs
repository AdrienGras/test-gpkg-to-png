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

    let (min_lon, min_lat, max_lon, max_lat) = (values[0], values[1], values[2], values[3]);

    // Validate that min < max for both lon and lat
    if min_lon >= max_lon {
        return Err(GpkgError::InvalidBbox(format!(
            "min_lon ({}) must be less than max_lon ({})",
            min_lon, max_lon
        )));
    }
    if min_lat >= max_lat {
        return Err(GpkgError::InvalidBbox(format!(
            "min_lat ({}) must be less than max_lat ({})",
            min_lat, max_lat
        )));
    }

    Ok(Bbox::new(min_lon, min_lat, max_lon, max_lat))
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

    #[test]
    fn test_parse_bbox_inverted() {
        // Test inverted longitude (max < min)
        let err = parse_bbox("-4.0,48.0,-4.5,48.5").unwrap_err();
        assert!(err.to_string().contains("min_lon"));
        assert!(err.to_string().contains("must be less than"));

        // Test inverted latitude (max < min)
        let err = parse_bbox("-4.5,48.5,-4.0,48.0").unwrap_err();
        assert!(err.to_string().contains("min_lat"));
        assert!(err.to_string().contains("must be less than"));

        // Test equal values (also invalid)
        let err = parse_bbox("-4.5,48.0,-4.5,48.5").unwrap_err();
        assert!(err.to_string().contains("min_lon"));
    }
}
