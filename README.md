# gpui-canvas

An interactive, GPU-accelerated 2D vector canvas and polygon annotation widget for [GPUI](https://github.com/zed-industries/zed) (Zed's GPU-accelerated desktop UI framework).

Designed for image/video annotation, boundary delineation, and interactive vector region definition.

## Features

- **Resolution-Independent Coordinates**: Stores vertices as normalized (0.0 to 1.0) coordinates so annotation polygons scale losslessly across window resizing, aspect-ratio shifts, and surface resolution changes.
- **Interactive Multi-Polygon Editing**:
  - **Draw Mode**: Click to place vertices, click first vertex or right-click to close.
  - **Select / Reshape Mode**: Drag vertices to reshape polygons; right-click to delete a vertex.
  - **View-Only Mode**: Clean read-only rendering without edit handles.
- **Hardware-Accelerated Vector Graphics**: Renders fills, strokes, and vertices via GPUI's `PathBuilder` and native GPU pipelines (Vulkan / Metal / Direct3D).
- **Serialization**: Native `serde` support for saving and loading zone configurations to/from JSON.

## Installation

Add `gpui-canvas` and `gpui` to your `Cargo.toml`:

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui-canvas = { git = "https://github.com/ceghap/gpui-canvas-" }
```

## Quick Start

```rust
use gpui::{prelude::*, App, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_platform::application;
use gpui_canvas::{InteractiveCanvas, CanvasMode};

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.0), px(600.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_cx| {
                    let mut canvas = InteractiveCanvas::new();
                    let idx = canvas.begin_new_zone("Region A", 0xef4444);
                    // Add normalized points (0.0 - 1.0)
                    if let Some(zone) = canvas.zones.get_mut(idx) {
                        zone.points.push(gpui_canvas::Point2D::new(0.2, 0.2));
                        zone.points.push(gpui_canvas::Point2D::new(0.8, 0.2));
                        zone.points.push(gpui_canvas::Point2D::new(0.5, 0.8));
                        zone.is_closed = true;
                    }
                    canvas.mode = CanvasMode::Select;
                    canvas
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
```

## Running the Example

Run the polygon editor demo:

```bash
cargo run --example polygon_editor
```

## License

Dual-licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.
