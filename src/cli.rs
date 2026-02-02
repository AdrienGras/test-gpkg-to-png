//! CLI argument parsing and validation.

use clap::Parser;
use std::path::PathBuf;

use crate::error::{GpkgError, Result};
use crate::math::Bbox;

/// Input file format
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum Format {
    Gpkg,
    Geojson,
}

/// Command line arguments for gpkg-to-png.
#[derive(Parser, Debug)]
#[command(name = "gpkg-to-png")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to the .gpkg file.
    pub input: PathBuf,

    /// Output directory.
    #[arg(short, long, default_value = ".")]
    pub output_dir: PathBuf,

    /// Bounding box: "minLon,minLat,maxLon,maxLat" (auto-detected from GPKG if not provided).
    #[arg(short, long)]
    pub bbox: Option<String>,

    /// Pixel size in degrees (mutually exclusive with --scale).
    #[arg(short, long)]
    pub resolution: Option<f64>,

    /// Scale in meters per pixel (mutually exclusive with --resolution).
    #[arg(short, long)]
    pub scale: Option<f64>,

    /// Fill color RGBA hex (e.g., "FF000080").
    #[arg(long, default_value = "FF000080")]
    pub fill: String,

    /// Stroke color RGB hex (e.g., "FF0000").
    #[arg(long, default_value = "FF0000")]
    pub stroke: String,

    /// Stroke width in pixels.
    #[arg(long, default_value = "1")]
    pub stroke_width: u32,

    /// Specific layer to render (default: all).
    #[arg(short, long)]
    pub layer: Option<String>,

    /// Input file format
    #[arg(short = 'f', long, value_enum)]
    pub format: Format,

    /// Output PNG filename (GeoJSON only, default: input filename)
    #[arg(long)]
    pub output_name: Option<String>,
}

/// Fully validated configuration object.
#[derive(Debug)]
pub struct Config {
    /// Path to the input GeoPackage.
    pub input: PathBuf,
    /// Path to the output directory.
    pub output_dir: PathBuf,
    /// Bounding box (None means auto-detect from GPKG).
    pub bbox: Option<Bbox>,
    /// Resolution in degrees per pixel.
    pub resolution: Option<f64>,
    /// Scale in meters per pixel.
    pub scale: Option<f64>,
    /// Fill color RGBA.
    pub fill: [u8; 4],
    /// Stroke color RGB.
    pub stroke: [u8; 3],
    /// Stroke width.
    pub stroke_width: u32,
    /// Optional layer name filter.
    pub layer: Option<String>,
    /// Output filename for GeoJSON (None for GPKG).
    pub output_name: Option<String>,
    /// Input format.
    pub format: Format,
}

impl Args {
    /// Validates arguments and converts them to a structured `Config`.
    ///
    /// Checks for mutually exclusive options and parses color hex strings.
    pub fn validate(self) -> Result<Config> {
        // Validate that at least one of resolution or scale is provided
        if self.resolution.is_none() && self.scale.is_none() {
            return Err(GpkgError::MissingResolutionOrScale);
        }

        // Validate that resolution and scale are mutually exclusive
        if self.resolution.is_some() && self.scale.is_some() {
            return Err(GpkgError::MutuallyExclusiveOptions(
                "resolution".to_string(),
                "scale".to_string(),
            ));
        }

        // Validate resolution if provided
        if let Some(res) = self.resolution {
            if res <= 0.0 {
                return Err(GpkgError::InvalidResolution(res));
            }
        }

        // Validate scale if provided
        if let Some(scale) = self.scale {
            if scale <= 0.0 {
                return Err(GpkgError::InvalidScale(scale));
            }
        }

        // Parse bbox if provided
        let bbox = self.bbox.as_ref().map(|s| parse_bbox(s)).transpose()?;

        // Parse colors
        let fill = parse_rgba(&self.fill)?;
        let stroke = parse_rgb(&self.stroke)?;

        // Validate format-specific options
        if matches!(self.format, Format::Geojson) && self.layer.is_some() {
            return Err(GpkgError::InvalidFormatOption(
                "--layer cannot be used with geojson format".to_string()
            ));
        }

        if matches!(self.format, Format::Gpkg) && self.output_name.is_some() {
            return Err(GpkgError::InvalidFormatOption(
                "--output-name can only be used with geojson format".to_string()
            ));
        }

        // Determine output name for GeoJSON
        let output_name = if matches!(self.format, Format::Geojson) {
            Some(
                self.output_name.clone().unwrap_or_else(|| {
                    self.input
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output")
                        .to_string()
                })
            )
        } else {
            None
        };

        Ok(Config {
            input: self.input,
            output_dir: self.output_dir,
            bbox,
            resolution: self.resolution,
            scale: self.scale,
            fill,
            stroke,
            stroke_width: self.stroke_width,
            layer: self.layer,
            output_name,
            format: self.format,
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

    fn create_test_args(resolution: Option<f64>, scale: Option<f64>, bbox: Option<&str>) -> Args {
        Args {
            input: PathBuf::from("test.gpkg"),
            output_dir: PathBuf::from("."),
            bbox: bbox.map(|s| s.to_string()),
            resolution,
            scale,
            fill: "FF000080".to_string(),
            stroke: "FF0000".to_string(),
            stroke_width: 1,
            layer: None,
            format: Format::Gpkg,
            output_name: None,
        }
    }

    #[test]
    fn test_validate_resolution_only() {
        let args = create_test_args(Some(0.001), None, Some("-4.5,48.0,-4.0,48.5"));
        let config = args.validate().unwrap();
        assert!(config.resolution.is_some());
        assert!(config.scale.is_none());
        assert!((config.resolution.unwrap() - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_validate_scale_only() {
        let args = create_test_args(None, Some(10.0), Some("-4.5,48.0,-4.0,48.5"));
        let config = args.validate().unwrap();
        assert!(config.resolution.is_none());
        assert!(config.scale.is_some());
        assert!((config.scale.unwrap() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_validate_neither_resolution_nor_scale() {
        let args = create_test_args(None, None, Some("-4.5,48.0,-4.0,48.5"));
        let err = args.validate().unwrap_err();
        assert!(err.to_string().contains("resolution"));
        assert!(err.to_string().contains("scale"));
    }

    #[test]
    fn test_validate_both_resolution_and_scale() {
        let args = create_test_args(Some(0.001), Some(10.0), Some("-4.5,48.0,-4.0,48.5"));
        let err = args.validate().unwrap_err();
        assert!(err.to_string().contains("mutually exclusive"));
    }

    #[test]
    fn test_validate_optional_bbox() {
        let args = create_test_args(Some(0.001), None, None);
        let config = args.validate().unwrap();
        assert!(config.bbox.is_none());
    }

    #[test]
    fn test_validate_invalid_scale() {
        let args = create_test_args(None, Some(-10.0), Some("-4.5,48.0,-4.0,48.5"));
        let err = args.validate().unwrap_err();
        assert!(err.to_string().contains("Scale must be positive"));
    }

    #[test]
    fn test_validate_invalid_resolution() {
        let args = create_test_args(Some(-0.001), None, Some("-4.5,48.0,-4.0,48.5"));
        let err = args.validate().unwrap_err();
        assert!(err.to_string().contains("Resolution must be positive"));
    }

    #[test]
    fn test_scale_to_resolution_conversion() {
        // Test the formula: resolution = scale / (111319.0 * cos(center_lat_radians))
        // At the equator (0 lat), cos(0) = 1, so resolution = scale / 111319.0
        // For 10 m/pixel at equator: resolution = 10 / 111319.0 ~= 0.0000898315
        let scale = 10.0;
        let center_lat: f64 = 0.0;
        let resolution = scale / (111319.0 * center_lat.to_radians().cos());
        assert!((resolution - 0.0000898315).abs() < 0.0000001);

        // At 48 degrees (roughly France), cos(48 deg) ~= 0.6691
        // resolution = 10 / (111319.0 * 0.6691) ~= 0.0001342
        let center_lat: f64 = 48.0;
        let resolution = scale / (111319.0 * center_lat.to_radians().cos());
        assert!((resolution - 0.0001342).abs() < 0.0001);
    }

    #[test]
    fn test_validate_geojson_with_layer_option() {
        let args = Args {
            input: PathBuf::from("test.geojson"),
            output_dir: PathBuf::from("."),
            bbox: Some("-4.5,48.0,-4.0,48.5".to_string()),
            resolution: Some(0.001),
            scale: None,
            fill: "FF000080".to_string(),
            stroke: "FF0000".to_string(),
            stroke_width: 1,
            layer: Some("test_layer".to_string()),
            format: Format::Geojson,
            output_name: None,
        };
        let err = args.validate().unwrap_err();
        assert!(err.to_string().contains("--layer cannot be used with geojson format"));
    }

    #[test]
    fn test_validate_geojson_default_output_name() {
        let args = Args {
            input: PathBuf::from("test.geojson"),
            output_dir: PathBuf::from("."),
            bbox: Some("-4.5,48.0,-4.0,48.5".to_string()),
            resolution: Some(0.001),
            scale: None,
            fill: "FF000080".to_string(),
            stroke: "FF0000".to_string(),
            stroke_width: 1,
            layer: None,
            format: Format::Geojson,
            output_name: None,
        };
        let config = args.validate().unwrap();
        assert_eq!(config.output_name, Some("test".to_string()));
    }

    #[test]
    fn test_validate_geojson_custom_output_name() {
        let args = Args {
            input: PathBuf::from("test.geojson"),
            output_dir: PathBuf::from("."),
            bbox: Some("-4.5,48.0,-4.0,48.5".to_string()),
            resolution: Some(0.001),
            scale: None,
            fill: "FF000080".to_string(),
            stroke: "FF0000".to_string(),
            stroke_width: 1,
            layer: None,
            format: Format::Geojson,
            output_name: Some("custom".to_string()),
        };
        let config = args.validate().unwrap();
        assert_eq!(config.output_name, Some("custom".to_string()));
    }

    #[test]
    fn test_validate_gpkg_with_output_name_option() {
        let args = Args {
            input: PathBuf::from("test.gpkg"),
            output_dir: PathBuf::from("."),
            bbox: Some("-4.5,48.0,-4.0,48.5".to_string()),
            resolution: Some(0.001),
            scale: None,
            fill: "FF000080".to_string(),
            stroke: "FF0000".to_string(),
            stroke_width: 1,
            layer: None,
            format: Format::Gpkg,
            output_name: Some("custom".to_string()),
        };
        let err = args.validate().unwrap_err();
        assert!(err.to_string().contains("--output-name can only be used with geojson format"));
    }
}
