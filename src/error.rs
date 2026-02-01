use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpkgError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[allow(dead_code)]
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

    #[error("Scale must be positive, got: {0}")]
    InvalidScale(f64),

    #[error("Either --resolution or --scale must be provided")]
    MissingResolutionOrScale,

    #[error("Options --{0} and --{1} are mutually exclusive")]
    MutuallyExclusiveOptions(String, String),

    #[error("Image dimensions too large: {width}x{height} pixels (max: {max})")]
    ImageTooLarge { width: u32, height: u32, max: u32 },

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
        assert_eq!(
            err.to_string(),
            "Layer 'foo' not found. Available layers: bar, baz"
        );
    }
}
