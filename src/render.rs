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
        self.draw_linestring(polygon.exterior().coords().copied(), stroke, width);

        // Draw interior rings (holes)
        for interior in polygon.interiors() {
            self.draw_linestring(interior.coords().copied(), stroke, width);
        }
    }

    /// Draw a linestring with given color and width
    fn draw_linestring(
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
