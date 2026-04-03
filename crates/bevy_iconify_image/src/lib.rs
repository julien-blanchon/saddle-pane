//! SVG-to-Image rasterization for Bevy, using usvg + tiny-skia.
//!
//! This is the **image backend** for [`bevy_iconify`]. It rasterizes SVG strings
//! into Bevy [`Image`] assets, rendered as standard [`ImageNode`] UI components.
//! Works correctly with `GlobalZIndex` and overflow clipping.
//!
//! Use this when you need icons inside Bevy UI panels. For world-space or
//! full-fidelity vector rendering, use `bevy_iconify_vello` instead.
//!
//! # Quick start
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_iconify_image::svg_to_image;
//!
//! const ICON: &str = bevy_iconify::svg!("lucide:settings", color = "#78797f");
//!
//! fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
//!     let handle = images.add(svg_to_image(ICON, 48).unwrap());
//!     commands.spawn((
//!         Node { width: Val::Px(24.0), height: Val::Px(24.0), ..default() },
//!         ImageNode::new(handle),
//!     ));
//! }
//! ```

use bevy::prelude::*;

/// Rasterize an SVG string to a Bevy [`Image`] at the given pixel size.
///
/// Parses with usvg, renders solid-color fills and strokes with tiny-skia.
/// The SVG is uniformly scaled to fit the target size, centered in the output.
///
/// Returns `None` if the SVG cannot be parsed or the pixmap cannot be allocated.
///
/// # Supported SVG features
///
/// - Path fills (solid color)
/// - Path strokes (solid color, line caps, line joins, miter limits)
/// - Nested groups with transforms
/// - Anti-aliasing
///
/// Gradients, filters, masks, and embedded images are ignored (not needed for
/// icon sets like Lucide, Material Design Icons, Heroicons, etc.).
pub fn svg_to_image(svg: &str, size: u32) -> Option<Image> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;

    // Uniform scale to fit SVG in target size, centered
    let svg_size = tree.size();
    let scale = (size as f32 / svg_size.width()).min(size as f32 / svg_size.height());
    let offset_x = (size as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (size as f32 - svg_size.height() * scale) / 2.0;
    let view_transform =
        tiny_skia::Transform::from_translate(offset_x, offset_y).post_scale(scale, scale);

    render_nodes(tree.root().children(), view_transform, &mut pixmap);

    // Convert premultiplied RGBA (tiny-skia output) → straight RGBA (Bevy expects)
    let mut data = pixmap.take();
    for pixel in data.chunks_exact_mut(4) {
        let a = pixel[3] as f32 / 255.0;
        if a > 0.0 {
            pixel[0] = (pixel[0] as f32 / a).min(255.0) as u8;
            pixel[1] = (pixel[1] as f32 / a).min(255.0) as u8;
            pixel[2] = (pixel[2] as f32 / a).min(255.0) as u8;
        }
    }

    Some(Image::new(
        bevy::render::render_resource::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ))
}

// ── Internal rendering ──────────────────────────────────────────────

fn render_nodes(
    nodes: &[usvg::Node],
    view_transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    for node in nodes {
        match node {
            usvg::Node::Group(group) => {
                render_nodes(group.children(), view_transform, pixmap);
            }
            usvg::Node::Path(path) => {
                // abs_transform includes all parent group transforms
                let transform = view_transform.pre_concat(path.abs_transform());
                render_path(path, transform, pixmap);
            }
            _ => {} // Image/Text not needed for icons
        }
    }
}

fn render_path(
    path: &usvg::Path,
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::Pixmap,
) {
    let data = path.data();

    if let Some(fill) = path.fill() {
        if let Some(paint) = solid_paint(fill.paint(), fill.opacity()) {
            let rule = match fill.rule() {
                usvg::FillRule::NonZero => tiny_skia::FillRule::Winding,
                usvg::FillRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
            };
            pixmap.fill_path(data, &paint, rule, transform, None);
        }
    }

    if let Some(stroke) = path.stroke() {
        if let Some(paint) = solid_paint(stroke.paint(), stroke.opacity()) {
            let sk_stroke = tiny_skia::Stroke {
                width: stroke.width().get(),
                line_cap: match stroke.linecap() {
                    usvg::LineCap::Butt => tiny_skia::LineCap::Butt,
                    usvg::LineCap::Round => tiny_skia::LineCap::Round,
                    usvg::LineCap::Square => tiny_skia::LineCap::Square,
                },
                line_join: match stroke.linejoin() {
                    usvg::LineJoin::Miter => tiny_skia::LineJoin::Miter,
                    usvg::LineJoin::MiterClip => tiny_skia::LineJoin::MiterClip,
                    usvg::LineJoin::Round => tiny_skia::LineJoin::Round,
                    usvg::LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
                },
                miter_limit: stroke.miterlimit().get(),
                ..tiny_skia::Stroke::default()
            };
            pixmap.stroke_path(data, &paint, &sk_stroke, transform, None);
        }
    }
}

fn solid_paint(paint: &usvg::Paint, opacity: usvg::Opacity) -> Option<tiny_skia::Paint<'static>> {
    match paint {
        usvg::Paint::Color(c) => {
            let a = (opacity.get() * 255.0) as u8;
            let mut p = tiny_skia::Paint::default();
            p.set_color(tiny_skia::Color::from_rgba8(c.red, c.green, c.blue, a));
            p.anti_alias = true;
            Some(p)
        }
        _ => None,
    }
}
