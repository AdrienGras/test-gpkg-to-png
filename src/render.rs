//! Raster rendering logic for GeoPackage polygons.
//!
//! This module implements a scanline rasterization algorithm for filling polygons
//! and uses Bresenham's algorithm for stroke rendering. It supports alpha blending
//! for overlapping geometries.

use geo::{Coord, MultiPolygon};
use image::{ImageBuffer, Rgba, RgbaImage};
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod edge;

use crate::error::{GpkgError, Result};
use crate::math::{calculate_dimensions, world_to_screen, Bbox};
use edge::{Edge, ScanlineTable};

const MAX_DIMENSION: u32 = 20000;

/// Render configuration for a layer.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Bounding box to render.
    pub bbox: Bbox,
    /// Resolution in degrees per pixel.
    pub resolution: f64,
    /// Fill color in RGBA format.
    pub fill: [u8; 4],
    /// Stroke color in RGB format.
    pub stroke: [u8; 3],
    /// Stroke width in pixels.
    pub stroke_width: u32,
}

/// Renderer that manages the output image buffer and rendering operations.
///
/// Uses an internal `Arc<Mutex<RgbaImage>>` to allow parallel rendering of
/// different geometries or image bands.
pub struct Renderer {
    config: RenderConfig,
    width: u32,
    height: u32,
    image: Arc<Mutex<RgbaImage>>,
}

impl Renderer {
    /// Create a new renderer with the given configuration.
    ///
    /// Validates that the resulting image dimensions don't exceed `MAX_DIMENSION`.
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
            image: Arc::new(Mutex::new(image)),
        })
    }

    /// Get image dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Render a MultiPolygon onto the image.
    ///
    /// This uses a scanline fill algorithm:
    /// 1. Build a Global Edge Table (GET) for all edges of the MultiPolygon.
    /// 2. Divide the image into horizontal bands for parallel processing.
    /// 3. For each band, iterate through scanlines using an Active Edge Table (AET).
    /// 4. Apply the Even-Odd rule to determine which pixels to fill.
    /// 5. Finally, draw the stroke if `stroke_width > 0`.
    pub fn render_multipolygon(&self, mp: &MultiPolygon<f64>) {
        // Build GET (Global Edge Table)
        let mut scanline_table = ScanlineTable::new(0, self.height);
        for polygon in mp {
            scanline_table.extract_from_polygon(
                polygon,
                &self.config.bbox,
                self.config.resolution,
                self.height,
            );
        }

        let num_bands = rayon::current_num_threads().max(1) * 4;
        let band_height = (self.height as usize).div_ceil(num_bands);

        (0..num_bands).into_par_iter().for_each(|band_idx| {
            let y_start = (band_idx * band_height) as i32;
            let y_end = ((band_idx + 1) * band_height).min(self.height as usize) as i32;

            if y_start >= y_end {
                return;
            }

            let mut active_edge_table: Vec<Edge> = Vec::new();
            let fill_color = Rgba(self.config.fill);

            for y in 0..y_end {
                // Add new edges from GET
                if y < self.height as i32 {
                    if let Some(new_edges) = scanline_table.entries.get(y as usize) {
                        for edge in new_edges {
                            active_edge_table.push(edge.clone());
                        }
                    }
                }

                // Remove edges where y_max == y
                active_edge_table.retain(|edge| edge.y_max > y);

                // For rows in our band, fill pixels
                if y >= y_start {
                    // Sort AET by x_current
                    active_edge_table
                        .sort_by(|a, b| a.x_current.partial_cmp(&b.x_current).unwrap());

                    // Fill intervals (Even-Odd rule)
                    let mut intersections = active_edge_table.iter().peekable();
                    if intersections.peek().is_some() {
                        let mut img = self.image.lock().unwrap();
                        while let (Some(e1), Some(e2)) =
                            (intersections.next(), intersections.next())
                        {
                            let x_start = (e1.x_current.round() as i32)
                                .max(0)
                                .min(self.width as i32 - 1)
                                as u32;
                            let x_end =
                                (e2.x_current.round() as i32).max(0).min(self.width as i32) as u32;

                            for x in x_start..x_end {
                                blend_pixel(&mut img, x, y as u32, fill_color);
                            }
                        }
                    }
                }

                // Update x_current for next scanline
                for edge in &mut active_edge_table {
                    edge.x_current += edge.inv_slope;
                }
            }
        });

        if self.config.stroke_width > 0 {
            mp.iter().par_bridge().for_each(|polygon| {
                self.render_polygon_stroke(polygon);
            });
        }
    }

    /// Draw the stroke (boundary) of a polygon.
    fn render_polygon_stroke(&self, polygon: &geo::Polygon<f64>) {
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
        &self,
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

    /// Draw a line segment using Bresenham's algorithm.
    ///
    /// This implementation supports thick lines by drawing a square of pixels
    /// around each point of the ideal line.
    fn draw_line(&self, from: (f64, f64), to: (f64, f64), color: Rgba<u8>, width: u32) {
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
                        let mut img = self.image.lock().unwrap();
                        blend_pixel(&mut img, px as u32, py as u32, color);
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

    /// Save the image to a PNG file
    pub fn save(&self, path: &Path) -> Result<()> {
        let img = self.image.lock().unwrap();
        img.save(path)?;
        Ok(())
    }
}

/// Blend a pixel with alpha compositing (Porter-Duff 'Over' operator).
///
/// This performs standard alpha blending of the `src` color over the `dst` color.
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
    fn test_render_simple_polygon() {
        let config = RenderConfig {
            bbox: Bbox::new(0.0, 0.0, 10.0, 10.0),
            resolution: 1.0,
            fill: [255, 0, 0, 255],
            stroke: [0, 0, 0],
            stroke_width: 0,
        };
        let renderer = Renderer::new(config).unwrap();

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
        let img = renderer.image.lock().unwrap();
        let center = img.get_pixel(5, 5);
        assert_eq!(center.0, [255, 0, 0, 255]);

        // Check corner pixel is transparent
        let corner = img.get_pixel(0, 0);
        assert_eq!(corner.0, [0, 0, 0, 0]);
    }
}
