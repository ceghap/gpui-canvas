use gpui::{
    div, prelude::*, px, rgb, size, App, Bounds, Context, Entity, Render, Window, WindowBounds,
    WindowOptions,
};
use gpui_canvas::{CanvasMode, InteractiveCanvas, Point2D};
use gpui_platform::application;

struct PolygonEditorApp {
    canvas: Entity<InteractiveCanvas>,
}

impl PolygonEditorApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|_cx| {
            let mut c = InteractiveCanvas::new();
            // Pre-seed an initial sample polygon
            let z_idx = c.begin_new_zone("Region A", 0xef4444);
            if let Some(zone) = c.zones.get_mut(z_idx) {
                zone.points.push(Point2D::new(0.15, 0.20));
                zone.points.push(Point2D::new(0.85, 0.20));
                zone.points.push(Point2D::new(0.75, 0.80));
                zone.points.push(Point2D::new(0.25, 0.80));
                zone.is_closed = true;
            }
            c.mode = CanvasMode::Select;
            c
        });

        Self { canvas }
    }
}

impl Render for PolygonEditorApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let canvas = self.canvas.clone();
        let (mode, zone_count) = {
            let c = canvas.read(cx);
            (c.mode, c.zones.len())
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x020617)) // Slate 950
            .text_color(rgb(0xf8fafc))
            // 1. Top Header & Interactive Toolbar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .bg(rgb(0x0f172a))
                    .border_b_1()
                    .border_color(rgb(0x1e293b))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(rgb(0x38bdf8))
                                    .child("gpui-canvas"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(rgb(0x1e293b))
                                    .text_color(rgb(0x94a3b8))
                                    .child(format!("{} Polygons", zone_count)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // Button: Add Red Polygon
                            .child(
                                div()
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .bg(rgb(0xdc2626))
                                    .hover(|s| s.bg(rgb(0xef4444)))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .child("+ Red Polygon")
                                    .on_mouse_down(
                                        gpui::MouseButton::Left,
                                        cx.listener({
                                            let canvas = canvas.clone();
                                            move |_this, _, _, cx| {
                                                canvas.update(cx, |c, cx| {
                                                    c.begin_new_zone("Region (Red)", 0xef4444);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            }
                                        }),
                                    ),
                            )
                            // Button: Add Green Polygon
                            .child(
                                div()
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .bg(rgb(0x059669))
                                    .hover(|s| s.bg(rgb(0x10b981)))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .child("+ Green Polygon")
                                    .on_mouse_down(
                                        gpui::MouseButton::Left,
                                        cx.listener({
                                            let canvas = canvas.clone();
                                            move |_this, _, _, cx| {
                                                canvas.update(cx, |c, cx| {
                                                    c.begin_new_zone("Region (Green)", 0x10b981);
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            }
                                        }),
                                    ),
                            )
                            // Button: Select / Reshape Mode
                            .child(
                                div()
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .bg(if mode == CanvasMode::Select {
                                        rgb(0x2563eb)
                                    } else {
                                        rgb(0x334155)
                                    })
                                    .hover(|s| s.bg(rgb(0x3b82f6)))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .child("Select / Reshape")
                                    .on_mouse_down(
                                        gpui::MouseButton::Left,
                                        cx.listener({
                                            let canvas = canvas.clone();
                                            move |_this, _, _, cx| {
                                                canvas.update(cx, |c, cx| {
                                                    c.mode = CanvasMode::Select;
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            }
                                        }),
                                    ),
                            )
                            // Button: Clear
                            .child(
                                div()
                                    .px_3()
                                    .py_1p5()
                                    .rounded_md()
                                    .bg(rgb(0x1e293b))
                                    .hover(|s| s.bg(rgb(0x334155)))
                                    .text_xs()
                                    .text_color(rgb(0xf87171))
                                    .cursor_pointer()
                                    .child("Clear")
                                    .on_mouse_down(
                                        gpui::MouseButton::Left,
                                        cx.listener({
                                            let canvas = canvas.clone();
                                            move |_this, _, _, cx| {
                                                canvas.update(cx, |c, cx| {
                                                    c.clear();
                                                    cx.notify();
                                                });
                                                cx.notify();
                                            }
                                        }),
                                    ),
                            ),
                    ),
            )
            // 2. Interactive Viewport Area
            .child(
                div()
                    .relative()
                    .flex_1()
                    .w_full()
                    .bg(rgb(0x090d16))
                    // Central background placeholder
                    .child(
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x1e293b))
                                    .child("[ Canvas Surface • Interactive 2D Vector Overlay ]"),
                            ),
                    )
                    // The Interactive Vector Canvas Element
                    .child(div().absolute().inset_0().child(canvas.clone())),
            )
            // 3. Status & Instructions Footer
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x0b1120))
                    .border_t_1()
                    .border_color(rgb(0x1e293b))
                    .text_xs()
                    .text_color(rgb(0x64748b))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(
                                div()
                                    .text_color(rgb(0x38bdf8))
                                    .child(match mode {
                                        CanvasMode::Draw => "✎ DRAW MODE: Click to add vertices. Click yellow vertex or Right-Click to close.",
                                        CanvasMode::Select => "⇲ SELECT MODE: Drag any vertex to reshape. Right-Click a vertex to remove it.",
                                        CanvasMode::ViewOnly => "🔒 VIEW ONLY",
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x94a3b8))
                            .child("Coordinates: Normalized 0.0 – 1.0 (Resolution-Independent)"),
                    ),
            )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("gpui-canvas • Interactive Polygon & Vector Editor".into()),
                    appears_transparent: false,
                    traffic_light_position: None,
                }),
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(PolygonEditorApp::new),
        )
        .unwrap();

        cx.activate(true);
    });
}
