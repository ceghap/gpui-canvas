use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    canvas, div, point, prelude::*, px, quad, rgb, Bounds, Context,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point,
    Render, Window,
};
use serde::{Deserialize, Serialize};

/// A 2D point with normalized coordinates (0.0 to 1.0) relative to canvas bounds.
///
/// Normalized coordinates guarantee that annotation zones scale losslessly
/// across arbitrary video resolutions, window resizes, and aspect ratios.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

impl Point2D {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Converts normalized coordinates into physical screen pixels based on canvas bounds.
    pub fn to_pixels(&self, bounds: Bounds<Pixels>) -> Point<Pixels> {
        let origin_x = bounds.origin.x.as_f32();
        let origin_y = bounds.origin.y.as_f32();
        let w = bounds.size.width.as_f32();
        let h = bounds.size.height.as_f32();

        point(
            px(origin_x + self.x * w),
            px(origin_y + self.y * h),
        )
    }

    /// Converts physical screen pixels into normalized coordinates (0.0 to 1.0).
    pub fn from_pixels(p: Point<Pixels>, bounds: Bounds<Pixels>) -> Self {
        let width = bounds.size.width.as_f32().max(1.0);
        let height = bounds.size.height.as_f32().max(1.0);

        let nx = ((p.x.as_f32() - bounds.origin.x.as_f32()) / width).clamp(0.0, 1.0);
        let ny = ((p.y.as_f32() - bounds.origin.y.as_f32()) / height).clamp(0.0, 1.0);

        Self { x: nx, y: ny }
    }

    /// Euclidean distance to another normalized point in pixel space.
    pub fn distance_px(&self, other: &Point2D, bounds: Bounds<Pixels>) -> f32 {
        let p1 = self.to_pixels(bounds);
        let p2 = other.to_pixels(bounds);
        let dx = p1.x.as_f32() - p2.x.as_f32();
        let dy = p1.y.as_f32() - p2.y.as_f32();
        (dx * dx + dy * dy).sqrt()
    }
}

/// A polygon region representing a security perimeter, detection zone, or curfew area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolygonZone {
    pub id: String,
    pub label: String,
    pub points: Vec<Point2D>,
    /// Color represented as 0xRRGGBB (e.g., 0xef4444 for red, 0x10b981 for emerald)
    pub color_rgb: u32,
    pub is_closed: bool,
}

impl PolygonZone {
    pub fn new(id: impl Into<String>, label: impl Into<String>, color_rgb: u32) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            points: Vec::new(),
            color_rgb,
            is_closed: false,
        }
    }
}

/// The interaction mode of the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasMode {
    /// Click on the canvas to add points to the active polygon.
    Draw,
    /// Click and drag vertices to modify existing polygons. Right-click to remove a vertex.
    Select,
    /// Read-only display of zones without edit handles.
    ViewOnly,
}

/// An interactive GPU-accelerated 2D vector canvas for polygon drawing and zone annotation.
pub struct InteractiveCanvas {
    pub zones: Vec<PolygonZone>,
    pub mode: CanvasMode,
    pub active_zone_index: Option<usize>,
    pub dragging_vertex: Option<(usize, usize)>, // (zone_idx, vertex_idx)
    pub hover_vertex: Option<(usize, usize)>,
    bounds_ref: Rc<Cell<Bounds<Pixels>>>,
}

impl Default for InteractiveCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractiveCanvas {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            mode: CanvasMode::Select,
            active_zone_index: None,
            dragging_vertex: None,
            hover_vertex: None,
            bounds_ref: Rc::new(Cell::new(Bounds::default())),
        }
    }

    /// Start drawing a new polygon zone with the given label and color.
    pub fn begin_new_zone(&mut self, label: impl Into<String>, color_rgb: u32) -> usize {
        let id = format!("zone-{}", self.zones.len() + 1);
        let zone = PolygonZone::new(id, label, color_rgb);
        self.zones.push(zone);
        let index = self.zones.len() - 1;
        self.active_zone_index = Some(index);
        self.mode = CanvasMode::Draw;
        index
    }

    /// Finish and close the active polygon if it has at least 3 vertices.
    pub fn finish_active_zone(&mut self) -> bool {
        if let Some(idx) = self.active_zone_index {
            if let Some(zone) = self.zones.get_mut(idx) {
                if zone.points.len() >= 3 {
                    zone.is_closed = true;
                    self.mode = CanvasMode::Select;
                    return true;
                }
            }
        }
        false
    }

    /// Clear all polygon zones.
    pub fn clear(&mut self) {
        self.zones.clear();
        self.active_zone_index = None;
        self.dragging_vertex = None;
        self.hover_vertex = None;
    }

    /// Remove a zone by index.
    pub fn remove_zone(&mut self, index: usize) {
        if index < self.zones.len() {
            self.zones.remove(index);
            if self.active_zone_index == Some(index) {
                self.active_zone_index = None;
                self.mode = CanvasMode::Select;
            }
        }
    }

    fn find_nearest_vertex(&self, click_norm: &Point2D, bounds: Bounds<Pixels>, threshold_px: f32) -> Option<(usize, usize)> {
        for (z_idx, zone) in self.zones.iter().enumerate() {
            for (v_idx, pt) in zone.points.iter().enumerate() {
                if pt.distance_px(click_norm, bounds) <= threshold_px {
                    return Some((z_idx, v_idx));
                }
            }
        }
        None
    }

    fn handle_mouse_down(&mut self, ev: &MouseDownEvent, cx: &mut Context<Self>) {
        if self.mode == CanvasMode::ViewOnly {
            return;
        }

        let bounds = self.bounds_ref.get();
        let click_norm = Point2D::from_pixels(ev.position, bounds);

        match self.mode {
            CanvasMode::Draw => {
                if let Some(z_idx) = self.active_zone_index {
                    if let Some(zone) = self.zones.get_mut(z_idx) {
                        // Check if clicking near first point to close polygon (threshold = 18px)
                        if zone.points.len() >= 3 {
                            let first = &zone.points[0];
                            if first.distance_px(&click_norm, bounds) <= 18.0 {
                                zone.is_closed = true;
                                self.mode = CanvasMode::Select;
                                cx.notify();
                                return;
                            }
                        }

                        // Add new vertex
                        zone.points.push(click_norm);
                        cx.notify();
                    }
                }
            }
            CanvasMode::Select => {
                if let Some((z_idx, v_idx)) = self.find_nearest_vertex(&click_norm, bounds, 16.0) {
                    self.active_zone_index = Some(z_idx);
                    self.dragging_vertex = Some((z_idx, v_idx));
                    cx.notify();
                }
            }
            CanvasMode::ViewOnly => {}
        }
    }

    fn handle_mouse_move(&mut self, ev: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.mode == CanvasMode::ViewOnly {
            return;
        }

        let bounds = self.bounds_ref.get();
        let norm_pos = Point2D::from_pixels(ev.position, bounds);

        if let Some((z_idx, v_idx)) = self.dragging_vertex {
            if let Some(zone) = self.zones.get_mut(z_idx) {
                if let Some(vertex) = zone.points.get_mut(v_idx) {
                    *vertex = norm_pos;
                    cx.notify();
                    return;
                }
            }
        }

        // Check hover over vertices
        let new_hover = self.find_nearest_vertex(&norm_pos, bounds, 14.0);
        if self.hover_vertex != new_hover {
            self.hover_vertex = new_hover;
            cx.notify();
        }
    }

    fn handle_mouse_up(&mut self, _ev: &MouseUpEvent, cx: &mut Context<Self>) {
        if self.dragging_vertex.is_some() {
            self.dragging_vertex = None;
            cx.notify();
        }
    }
}

impl Render for InteractiveCanvas {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.mode;
        let active_zone_idx = self.active_zone_index;
        let hover_vertex = self.hover_vertex;
        let dragging_vertex = self.dragging_vertex;

        let bounds_cell = self.bounds_ref.clone();
        let zones = self.zones.clone();

        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    this.handle_mouse_down(ev, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    // Right click deletes vertex in select mode, or finishes zone in draw mode
                    if this.mode == CanvasMode::Draw {
                        this.finish_active_zone();
                        cx.notify();
                    } else if this.mode == CanvasMode::Select {
                        let bounds = this.bounds_ref.get();
                        let click_norm = Point2D::from_pixels(ev.position, bounds);
                        if let Some((z_idx, v_idx)) = this.find_nearest_vertex(&click_norm, bounds, 16.0) {
                            if let Some(zone) = this.zones.get_mut(z_idx) {
                                if zone.points.len() > 3 {
                                    zone.points.remove(v_idx);
                                    cx.notify();
                                }
                            }
                        }
                    }
                }),
            )
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _window, cx| {
                this.handle_mouse_move(ev, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseUpEvent, _window, cx| {
                    this.handle_mouse_up(ev, cx);
                }),
            )
            .child(
                canvas(
                    move |bounds, _window, _cx| {
                        bounds_cell.set(bounds);
                    },
                    move |bounds, _state, window, _cx| {
                        for (z_idx, zone) in zones.iter().enumerate() {
                            if zone.points.is_empty() {
                                continue;
                            }

                            let is_active = Some(z_idx) == active_zone_idx;
                            let hex = zone.color_rgb;
                            let base_color = rgb(hex);

                            let pixel_points: Vec<Point<Pixels>> = zone
                                .points
                                .iter()
                                .map(|pt| pt.to_pixels(bounds))
                                .collect();

                            // 1. Draw Filled Polygon (if at least 3 points and closed)
                            if pixel_points.len() >= 3 && zone.is_closed {
                                let mut fill_builder = PathBuilder::fill();
                                fill_builder.move_to(pixel_points[0]);
                                for pt in &pixel_points[1..] {
                                    fill_builder.line_to(*pt);
                                }
                                fill_builder.close();

                                if let Ok(fill_path) = fill_builder.build() {
                                    let fill_alpha = if is_active { 0.35 } else { 0.20 };
                                    window.paint_path(fill_path, base_color.alpha(fill_alpha));
                                }
                            }

                            // 2. Draw Polygon Perimeter Stroke
                            if pixel_points.len() >= 2 {
                                let stroke_width = if is_active { px(2.5) } else { px(1.5) };
                                let mut stroke_builder = PathBuilder::stroke(stroke_width);
                                stroke_builder.move_to(pixel_points[0]);
                                for pt in &pixel_points[1..] {
                                    stroke_builder.line_to(*pt);
                                }
                                if zone.is_closed {
                                    stroke_builder.close();
                                }

                                if let Ok(stroke_path) = stroke_builder.build() {
                                    let stroke_alpha = if is_active { 0.95 } else { 0.70 };
                                    window.paint_path(stroke_path, base_color.alpha(stroke_alpha));
                                }
                            }

                            // 3. Draw Vertex Handles (if not in ViewOnly mode)
                            if mode != CanvasMode::ViewOnly {
                                for (v_idx, pt) in pixel_points.iter().enumerate() {
                                    let is_v_hover = hover_vertex == Some((z_idx, v_idx));
                                    let is_v_drag = dragging_vertex == Some((z_idx, v_idx));
                                    let is_first = v_idx == 0 && mode == CanvasMode::Draw && !zone.is_closed && pixel_points.len() >= 3;

                                    let handle_diameter = if is_v_drag || is_v_hover || is_first {
                                        px(12.0)
                                    } else {
                                        px(8.0)
                                    };

                                    let handle_bounds = Bounds {
                                        origin: point(pt.x - handle_diameter / 2.0, pt.y - handle_diameter / 2.0),
                                        size: gpui::size(handle_diameter, handle_diameter),
                                    };

                                    let handle_bg = if is_first {
                                        rgb(0xfbbf24) // Yellow highlight for close target
                                    } else if is_v_drag || is_v_hover {
                                        rgb(0x38bdf8) // Blue highlight
                                    } else {
                                        base_color
                                    };

                                    window.paint_quad(quad(
                                        handle_bounds,
                                        px(3.0),
                                        handle_bg,
                                        px(1.5),
                                        gpui::white(),
                                        Default::default(),
                                    ));
                                }
                            }
                        }
                    },
                )
                .size_full(),
            )
    }
}
