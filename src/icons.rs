//! SVG icon support for saddle_pane, powered by bevy_iconify + bevy_iconify_image.
//!
//! Pre-defined icon constants are available as `&'static str` SVG strings.
//! Use [`spawn_pane_icon_handle`] with a pre-rasterized `Handle<Image>`, or
//! the builder API's `.icon()` method.
//!
//! For custom icons, depend on `bevy_iconify` directly:
//! ```ignore
//! const MY_ICON: &str = bevy_iconify::svg!("lucide:star", color = "#ffcc00");
//! ```

use bevy::prelude::*;

// Re-export the rasterization function from the shared crate.
pub use bevy_iconify_image::svg_to_image;

// ══════════════════════════════════════════════════════════════════════
// Pre-defined icon SVG constants (compile-time via bevy_iconify)
// ══════════════════════════════════════════════════════════════════════

/// Crosshair icon — joystick controls, target indicators.
pub const ICON_CROSSHAIR: &str = bevy_iconify::svg!("lucide:crosshair", color = "#7aacdf");

/// Move icon — 4-directional arrows.
pub const ICON_MOVE: &str = bevy_iconify::svg!("lucide:move", color = "#7aacdf");

/// Folder icon — file browser, directory navigation.
pub const ICON_FOLDER: &str = bevy_iconify::svg!("lucide:folder-open", color = "#78797f");

/// Reset icon — counter-clockwise rotation arrow.
pub const ICON_RESET: &str = bevy_iconify::svg!("lucide:rotate-ccw", color = "#b0b2b8");

/// Eye icon — visibility toggles.
pub const ICON_EYE: &str = bevy_iconify::svg!("lucide:eye", color = "#78797f");

/// Eye-off icon — hidden/disabled visibility.
pub const ICON_EYE_OFF: &str = bevy_iconify::svg!("lucide:eye-off", color = "#5a5b60");

/// Palette icon — color controls.
pub const ICON_PALETTE: &str = bevy_iconify::svg!("lucide:palette", color = "#78797f");

/// Chevron down — expanded folders/sections.
pub const ICON_CHEVRON_DOWN: &str =
    bevy_iconify::svg!("lucide:chevron-down", color = "#78797f");

/// Chevron right — collapsed folders/sections.
pub const ICON_CHEVRON_RIGHT: &str =
    bevy_iconify::svg!("lucide:chevron-right", color = "#78797f");

/// Settings/gear icon.
pub const ICON_SETTINGS: &str = bevy_iconify::svg!("lucide:settings", color = "#78797f");

/// Sliders icon — general tweaking/controls.
pub const ICON_SLIDERS: &str =
    bevy_iconify::svg!("lucide:sliders-horizontal", color = "#78797f");

/// Save icon.
pub const ICON_SAVE: &str = bevy_iconify::svg!("lucide:save", color = "#b0b2b8");

/// Upload/load icon.
pub const ICON_UPLOAD: &str = bevy_iconify::svg!("lucide:upload", color = "#b0b2b8");

/// Spline/bezier icon.
pub const ICON_SPLINE: &str = bevy_iconify::svg!("lucide:spline", color = "#78797f");

/// Grid icon.
pub const ICON_GRID: &str = bevy_iconify::svg!("lucide:grid-2x2", color = "#78797f");

/// Monitor/activity icon.
pub const ICON_ACTIVITY: &str = bevy_iconify::svg!("lucide:activity", color = "#78797f");

/// Bug/debug icon.
pub const ICON_BUG: &str = bevy_iconify::svg!("lucide:bug", color = "#78797f");

/// Grip/drag-handle icon — vertical dots for dragging.
pub const ICON_GRIP_VERTICAL: &str =
    bevy_iconify::svg!("lucide:grip-vertical", color = "#5c5d64");

// ══════════════════════════════════════════════════════════════════════
// Icon spawning
// ══════════════════════════════════════════════════════════════════════

/// Spawn an icon as a UI `ImageNode` with a pre-rasterized handle. Renders immediately.
pub fn spawn_pane_icon_handle(
    parent: &mut ChildSpawnerCommands,
    handle: Handle<Image>,
    size: f32,
) -> Entity {
    parent
        .spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                min_width: Val::Px(size),
                min_height: Val::Px(size),
                flex_shrink: 0.0,
                ..default()
            },
            ImageNode::new(handle),
        ))
        .id()
}

/// Marker component for icon nodes awaiting deferred rasterization (fallback path).
#[derive(Component, Clone, Debug)]
pub struct PaneIconPlaceholder(pub String);

/// Spawn an SVG icon with deferred rasterization (for use by external plugins).
pub fn spawn_pane_icon(parent: &mut ChildSpawnerCommands, svg: &str, size: f32) -> Entity {
    parent
        .spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                min_width: Val::Px(size),
                min_height: Val::Px(size),
                flex_shrink: 0.0,
                ..default()
            },
            PaneIconPlaceholder(svg.to_string()),
        ))
        .id()
}

/// System: resolve [`PaneIconPlaceholder`] into [`ImageNode`] components.
pub(crate) fn resolve_pane_icons(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    q: Query<(Entity, &PaneIconPlaceholder)>,
) {
    for (entity, placeholder) in &q {
        if let Some(image) = svg_to_image(&placeholder.0, 48) {
            let handle = images.add(image);
            commands
                .entity(entity)
                .insert(ImageNode::new(handle))
                .remove::<PaneIconPlaceholder>();
        } else {
            warn!("Failed to rasterize SVG icon");
            commands.entity(entity).remove::<PaneIconPlaceholder>();
        }
    }
}

/// Recursively collect all icon SVG strings from a layout item tree.
pub(crate) fn collect_icon_svgs(
    items: &[crate::builder::LayoutItem],
    visitor: &mut dyn FnMut(&str),
) {
    use crate::builder::{ControlSpec, LayoutItem};
    for item in items {
        match item {
            LayoutItem::Control(spec) => {
                let icon = match spec {
                    ControlSpec::Slider { icon, .. }
                    | ControlSpec::Toggle { icon, .. }
                    | ControlSpec::Button { icon, .. }
                    | ControlSpec::Number { icon, .. }
                    | ControlSpec::Text { icon, .. }
                    | ControlSpec::Select { icon, .. }
                    | ControlSpec::Color { icon, .. }
                    | ControlSpec::Custom { icon, .. } => icon.as_deref(),
                    _ => None,
                };
                if let Some(svg) = icon {
                    visitor(svg);
                }
            }
            LayoutItem::Folder { items, .. } => collect_icon_svgs(items, visitor),
            LayoutItem::TabGroup { tabs, .. } => {
                for tab in tabs {
                    collect_icon_svgs(&tab.items, visitor);
                }
            }
        }
    }
}
