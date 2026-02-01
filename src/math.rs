//! Geometric utilities and coordinate transformations.

/// Bounding box in WGS84 coordinates (longitude, latitude).
#[derive(Debug, Clone, Copy)]
pub struct Bbox {
    /// Minimum longitude (degrees).
    pub min_lon: f64,
    /// Minimum latitude (degrees).
    pub min_lat: f64,
    /// Maximum longitude (degrees).
    pub max_lon: f64,
    /// Maximum latitude (degrees).
    pub max_lat: f64,
}

impl Bbox {
    /// Creates a new bounding box.
    pub fn new(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Self {
        Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        }
    }

    /// Returns the width of the bbox in degrees.
    pub fn width(&self) -> f64 {
        self.max_lon - self.min_lon
    }

    /// Returns the height of the bbox in degrees.
    pub fn height(&self) -> f64 {
        self.max_lat - self.min_lat
    }
}

/// Calculate image dimensions (width, height) from bbox and resolution.
pub fn calculate_dimensions(bbox: &Bbox, resolution: f64) -> (u32, u32) {
    let width = (bbox.width() / resolution).ceil() as u32;
    let height = (bbox.height() / resolution).ceil() as u32;
    (width, height)
}

/// Convert WGS84 coordinates to pixel coordinates
/// Y is inverted for image coordinate system (0,0 at top-left)
pub fn world_to_screen(
    lon: f64,
    lat: f64,
    bbox: &Bbox,
    resolution: f64,
    height: u32,
) -> (f64, f64) {
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
