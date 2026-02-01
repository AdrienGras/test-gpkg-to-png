//! Scanline edge generation and management.
//!
//! This module provides the `Edge` and `ScanlineTable` structures used by the
//! scanline fill algorithm to rasterize polygons efficiently.

use crate::math::{world_to_screen, Bbox};
use geo::CoordsIter;

/// Represents an edge in the scanline algorithm.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Maximum Y coordinate of the edge (scanning goes from y_min to y_max).
    pub y_max: i32,
    /// Current X coordinate at the starting scanline.
    pub x_current: f64,
    /// Reciprocal of the slope (dx/dy). Used to update x_current for each new scanline.
    pub inv_slope: f64,
}

impl Edge {
    /// Creates a new `Edge` from two points.
    ///
    /// Returns `None` if the edge is horizontal, as horizontal edges are
    /// handled implicitly by the scanline algorithm.
    pub fn new(p1: (f64, f64), p2: (f64, f64)) -> Option<Self> {
        let (_, y1) = p1;
        let (_, y2) = p2;

        if (y1 - y2).abs() < 1e-9 {
            return None; // Ignorer les segments horizontaux
        }

        let (p_start, p_end) = if y1 < y2 { (p1, p2) } else { (p2, p1) };
        let inv_slope = (p_end.0 - p_start.0) / (p_end.1 - p_start.1);

        Some(Edge {
            y_max: p_end.1.round() as i32,
            x_current: p_start.0,
            inv_slope,
        })
    }
}

/// A Global Edge Table (GET) organized by scanline.
pub struct ScanlineTable {
    /// Minimum Y coordinate in screen space.
    pub y_min: i32,
    /// Vector of edges starting at each scanline. Indexed by `y - y_min`.
    pub entries: Vec<Vec<Edge>>,
}

impl ScanlineTable {
    /// Creates a new empty `ScanlineTable`.
    pub fn new(y_min: i32, height: u32) -> Self {
        ScanlineTable {
            y_min,
            entries: (0..height).map(|_| Vec::new()).collect(),
        }
    }

    /// Adds an edge starting at a specific scanline.
    pub fn add_edge(&mut self, y_start: i32, edge: Edge) {
        let idx = (y_start - self.y_min) as usize;
        if idx < self.entries.len() {
            self.entries[idx].push(edge);
        }
    }

    /// Extracts all edges from a polygon and adds them to the table.
    ///
    /// This handles both the exterior ring and any interior holes.
    pub fn extract_from_polygon(
        &mut self,
        polygon: &geo::Polygon<f64>,
        bbox: &Bbox,
        resolution: f64,
        img_height: u32,
    ) {
        self.extract_from_ring(polygon.exterior(), bbox, resolution, img_height);
        for interior in polygon.interiors() {
            self.extract_from_ring(interior, bbox, resolution, img_height);
        }
    }

    /// Extracts edges from a single ring (exterior or interior).
    fn extract_from_ring(
        &mut self,
        ring: &geo::LineString<f64>,
        bbox: &Bbox,
        resolution: f64,
        img_height: u32,
    ) {
        if ring.coords_count() < 3 {
            return;
        }

        let coords: Vec<(f64, f64)> = ring
            .coords()
            .map(|c| world_to_screen(c.x, c.y, bbox, resolution, img_height))
            .collect();

        for i in 0..coords.len() {
            let p1 = coords[i];
            let p2 = coords[(i + 1) % coords.len()];

            if let Some(edge) = Edge::new(p1, p2) {
                let y_start = p1.1.min(p2.1).round() as i32;
                self.add_edge(y_start, edge);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Bbox;
    use geo::{coord, LineString, Polygon};

    #[test]
    fn test_edge_creation() {
        let p1 = (10.0, 10.0);
        let p2 = (20.0, 20.0);
        let edge = Edge::new(p1, p2).unwrap();
        assert_eq!(edge.y_max, 20);
        assert_eq!(edge.x_current, 10.0);
        assert_eq!(edge.inv_slope, 1.0);

        let horizontal = Edge::new((10.0, 10.0), (20.0, 10.0));
        assert!(horizontal.is_none());
    }

    #[test]
    fn test_scanline_table_extraction() {
        let bbox = Bbox::new(0.0, 0.0, 10.0, 10.0);
        let resolution = 1.0;
        let img_height = 10;

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

        let mut st = ScanlineTable::new(0, img_height);
        st.extract_from_polygon(&polygon, &bbox, resolution, img_height);

        // Polygons coordinates in screen space with world_to_screen:
        // x = (lon - 0.0) / 1.0
        // y = 10.0 - ((lat - 0.0) / 1.0)
        // (2,2) -> (2, 8)
        // (8,2) -> (8, 8)
        // (8,8) -> (8, 2)
        // (2,8) -> (2, 2)

        // Verticals are (2,8)-(2,2) and (8,8)-(8,2)
        // Edge 1: start Y=2, end Y=8, x=2, slope=0
        // Edge 2: start Y=2, end Y=8, x=8, slope=0

        assert_eq!(st.entries[2].len(), 2);
        assert_eq!(st.entries[2][0].x_current, 8.0); // depends on order
        assert_eq!(st.entries[2][1].x_current, 2.0);
    }
}
