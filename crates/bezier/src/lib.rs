//! # saddle_pane_bezier
//!
//! Cubic bezier curve editor plugin for [saddle_pane](https://github.com/your/saddle_pane).
//!
//! Visual curve editor with two draggable control points, preset curves,
//! and numeric X1/Y1/X2/Y2 fields. The curve is rendered to a texture for
//! a solid anti-aliased line.

use std::any::Any;

use bevy::asset::embedded_asset;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::picking::events::{Drag, Pointer};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;

use saddle_pane::controls::{PaneControlMeta, PaneValue, pane_font, spawn_label};
use saddle_pane::events::PaneChanged;
use saddle_pane::prelude::{PaneControlPlugin, PaneControlRegistry, PaneCustomValue, PaneSystems};
use saddle_pane::registry::{ControlConfig, CustomValueBox};
use saddle_pane::store::PaneStore;

const STYLE_PATH: &str = "embedded://saddle_pane_bezier/style/bezier.css";
const TEX_W: u32 = 256;
const TEX_H: u32 = 160;

// ══════════════════════════════════════════════════════════════════════
// Public types
// ══════════════════════════════════════════════════════════════════════

/// Value produced by the bezier control — the four control coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct BezierValue {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl BezierValue {
    pub fn as_array(&self) -> [f64; 4] {
        [self.x1, self.y1, self.x2, self.y2]
    }
}

impl PaneCustomValue for BezierValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn PaneCustomValue> {
        Box::new(self.clone())
    }
    fn eq_box(&self, other: &dyn PaneCustomValue) -> bool {
        other
            .as_any()
            .downcast_ref::<BezierValue>()
            .is_some_and(|o| o == self)
    }
}

/// Component storing the bezier control state.
#[derive(Component, Clone, Debug)]
pub struct BezierControl {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// Named preset curves.
pub const PRESETS: &[(&str, [f64; 4])] = &[
    ("linear", [0.0, 0.0, 1.0, 1.0]),
    ("ease", [0.25, 0.1, 0.25, 1.0]),
    ("ease-in", [0.42, 0.0, 1.0, 1.0]),
    ("ease-out", [0.0, 0.0, 0.58, 1.0]),
    ("ease-in-out", [0.42, 0.0, 0.58, 1.0]),
];

// ══════════════════════════════════════════════════════════════════════
// Internal markers
// ══════════════════════════════════════════════════════════════════════

#[derive(Component, Clone, Debug)]
struct BezierCP1;

#[derive(Component, Clone, Debug)]
struct BezierCP2;

#[derive(Component, Clone, Debug)]
struct BezierCanvas;

/// Links the canvas image entity to its texture handle.
#[derive(Component, Clone, Debug)]
struct BezierTexHandle(Handle<Image>);

#[derive(Component, Clone, Debug, Default)]
struct BezierX1Text;
#[derive(Component, Clone, Debug, Default)]
struct BezierY1Text;
#[derive(Component, Clone, Debug, Default)]
struct BezierX2Text;
#[derive(Component, Clone, Debug, Default)]
struct BezierY2Text;

#[derive(Component, Clone, Debug, Default)]
struct BezierPresetText;

#[derive(Component, Clone, Debug, Default)]
struct BezierPresetIndex(usize);

// ══════════════════════════════════════════════════════════════════════
// Plugin
// ══════════════════════════════════════════════════════════════════════

pub struct PaneBezierPlugin;

impl Plugin for PaneBezierPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "style/bezier.css");

        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "bezier",
            build: build_systems,
            spawn: spawn_bezier_ui,
            default_value: bezier_default_value,
        });

        build_systems(app);
    }
}

fn build_systems(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update_bezier_display.in_set(PaneSystems::Display),
    );
    app.add_systems(PostUpdate, sync_bezier_to_store.in_set(PaneSystems::Sync));
}

fn bezier_default_value(config: &ControlConfig) -> Option<PaneValue> {
    let x1 = config.get_float("x1").unwrap_or(0.25);
    let y1 = config.get_float("y1").unwrap_or(0.1);
    let x2 = config.get_float("x2").unwrap_or(0.25);
    let y2 = config.get_float("y2").unwrap_or(1.0);
    Some(PaneValue::Custom(CustomValueBox(Box::new(BezierValue {
        x1,
        y1,
        x2,
        y2,
    }))))
}

// ══════════════════════════════════════════════════════════════════════
// Texture creation & drawing
// ══════════════════════════════════════════════════════════════════════

fn create_bezier_texture() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: TEX_W,
            height: TEX_H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::nearest());
    image
}

/// Draw the complete bezier visualization onto the texture using set_color_at.
fn draw_bezier_to_texture(image: &mut Image, ctrl: &BezierControl) {
    let w = TEX_W;
    let h = TEX_H;
    let margin = 0.06;
    let usable = 1.0 - 2.0 * margin;

    // Clear to transparent
    for y in 0..h {
        for x in 0..w {
            let _ = image.set_color_at(x, y, Color::linear_rgba(0.0, 0.0, 0.0, 0.0));
        }
    }

    let map_x = |x: f64| -> f64 { (margin + x.clamp(0.0, 1.0) * usable) * w as f64 };
    let map_y = |y: f64| -> f64 {
        let normalized = (y + 0.5) / 2.0;
        (margin + (1.0 - normalized.clamp(0.0, 1.0)) * usable) * h as f64
    };

    let ref_color = Color::linear_rgba(1.0, 1.0, 1.0, 0.07);
    let handle_color = Color::linear_rgba(0.29, 0.44, 0.65, 0.5);
    let curve_color = Color::linear_rgba(1.0, 1.0, 1.0, 0.94);

    // Draw reference diagonal (faint)
    draw_thick_line(
        image,
        w,
        h,
        map_x(0.0),
        map_y(0.0),
        map_x(1.0),
        map_y(1.0),
        ref_color,
        1.0,
    );

    // Draw handle lines
    draw_thick_line(
        image,
        w,
        h,
        map_x(0.0),
        map_y(0.0),
        map_x(ctrl.x1),
        map_y(ctrl.y1),
        handle_color,
        1.0,
    );
    draw_thick_line(
        image,
        w,
        h,
        map_x(ctrl.x2),
        map_y(ctrl.y2),
        map_x(1.0),
        map_y(1.0),
        handle_color,
        1.0,
    );

    // Draw the bezier curve — sample densely and draw thick dots
    let steps = 800;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let bx = cubic_bezier(t, 0.0, ctrl.x1, ctrl.x2, 1.0);
        let by = cubic_bezier(t, 0.0, ctrl.y1, ctrl.y2, 1.0);
        let px = map_x(bx);
        let py = map_y(by);
        draw_dot(image, w, h, px, py, curve_color, 1.3);
    }
}

/// Draw a filled circle (dot) with anti-aliasing at sub-pixel position.
fn draw_dot(image: &mut Image, w: u32, h: u32, cx: f64, cy: f64, color: Color, radius: f64) {
    let r_ceil = (radius + 1.0).ceil() as i32;
    let ix_min = (cx as i32 - r_ceil).max(0);
    let ix_max = (cx as i32 + r_ceil).min(w as i32 - 1);
    let iy_min = (cy as i32 - r_ceil).max(0);
    let iy_max = (cy as i32 + r_ceil).min(h as i32 - 1);

    let LinearRgba {
        red,
        green,
        blue,
        alpha,
    } = color.to_linear();

    for iy in iy_min..=iy_max {
        for ix in ix_min..=ix_max {
            let dist = ((ix as f64 + 0.5 - cx).powi(2) + (iy as f64 + 0.5 - cy).powi(2)).sqrt();
            let aa = ((radius + 0.5 - dist) / 1.0).clamp(0.0, 1.0) as f32;
            if aa > 0.01 {
                let a = aa * alpha;
                if let Ok(existing) = image.get_color_at(ix as u32, iy as u32) {
                    let existing = existing.to_linear();
                    let inv = 1.0 - a;
                    let blended = Color::linear_rgba(
                        existing.red * inv + red * a,
                        existing.green * inv + green * a,
                        existing.blue * inv + blue * a,
                        (existing.alpha + a).min(1.0),
                    );
                    let _ = image.set_color_at(ix as u32, iy as u32, blended);
                }
            }
        }
    }
}

/// Draw a line made of overlapping dots.
fn draw_thick_line(
    image: &mut Image,
    w: u32,
    h: u32,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: Color,
    thickness: f64,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let dist = (dx * dx + dy * dy).sqrt();
    let steps = (dist * 2.0).ceil().max(1.0) as usize;

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let px = x0 + dx * t;
        let py = y0 + dy * t;
        draw_dot(image, w, h, px, py, color, thickness * 0.5);
    }
}

// ══════════════════════════════════════════════════════════════════════
// Spawn
// ══════════════════════════════════════════════════════════════════════

fn spawn_bezier_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let x1 = config.get_float("x1").unwrap_or(0.25);
    let y1 = config.get_float("y1").unwrap_or(0.1);
    let x2 = config.get_float("x2").unwrap_or(0.25);
    let y2 = config.get_float("y2").unwrap_or(1.0);

    let ctrl = BezierControl { x1, y1, x2, y2 };
    let preset_name = find_preset_name(&ctrl).unwrap_or("custom");

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            ctrl,
            BezierPresetIndex(0),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Header: label + preset button
            row.spawn((Node::default(), ClassList::new("pane-bezier-header")))
                .with_children(|header| {
                    spawn_label(header, &meta.label);

                    header
                        .spawn((
                            Node::default(),
                            Interaction::default(),
                            bevy_ui_widgets::Button,
                            ClassList::new("pane-bezier-preset-btn"),
                            bevy_ui_widgets::observe(on_preset_cycle),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(preset_name),
                                pane_font(10.0),
                                ClassList::new("pane-bezier-preset-text"),
                                BezierPresetText,
                            ));
                        });
                });

            // Canvas: a container with the texture image + overlay CP handles
            row.spawn((
                Node::default(),
                ClassList::new("pane-bezier-canvas"),
                BezierCanvas,
            ))
            .with_children(|canvas| {
                // The curve texture image — will be set up by init system
                canvas.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BezierTexHandle(Handle::default()),
                ));

                // CP1 draggable point (absolute positioned on top of texture)
                canvas.spawn((
                    Node::default(),
                    Interaction::default(),
                    ClassList::new("pane-bezier-cp"),
                    BezierCP1,
                    InlineStyle::default(),
                    bevy_ui_widgets::observe(on_cp1_drag),
                ));

                // CP2 draggable point
                canvas.spawn((
                    Node::default(),
                    Interaction::default(),
                    ClassList::new("pane-bezier-cp"),
                    BezierCP2,
                    InlineStyle::default(),
                    bevy_ui_widgets::observe(on_cp2_drag),
                ));
            });

            // Value fields: X1, Y1, X2, Y2
            row.spawn((Node::default(), ClassList::new("pane-bezier-fields")))
                .with_children(|fields| {
                    spawn_field(fields, "X1", format!("{x1:.2}"), BezierX1Text);
                    spawn_field(fields, "Y1", format!("{y1:.2}"), BezierY1Text);
                    spawn_field(fields, "X2", format!("{x2:.2}"), BezierX2Text);
                    spawn_field(fields, "Y2", format!("{y2:.2}"), BezierY2Text);
                });
        });

    row_entity
}

fn spawn_field(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: String,
    marker: impl Component,
) {
    parent
        .spawn((Node::default(), ClassList::new("pane-bezier-field-group")))
        .with_children(|g| {
            g.spawn((
                Text::new(label),
                pane_font(9.0),
                ClassList::new("pane-bezier-field-label"),
            ));
            g.spawn((Node::default(), ClassList::new("pane-bezier-field-value")))
                .with_children(|v| {
                    v.spawn((
                        Text::new(value),
                        pane_font(9.0),
                        ClassList::new("pane-bezier-field-text"),
                        marker,
                    ));
                });
        });
}

// ══════════════════════════════════════════════════════════════════════
// Interaction
// ══════════════════════════════════════════════════════════════════════

fn on_cp1_drag(
    ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    q_computed: Query<&ComputedNode>,
    mut q_row: Query<&mut BezierControl>,
) {
    handle_cp_drag(
        ev.entity,
        ev.event().delta,
        true,
        &q_parent,
        &q_computed,
        &mut q_row,
    );
}

fn on_cp2_drag(
    ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    q_computed: Query<&ComputedNode>,
    mut q_row: Query<&mut BezierControl>,
) {
    handle_cp_drag(
        ev.entity,
        ev.event().delta,
        false,
        &q_parent,
        &q_computed,
        &mut q_row,
    );
}

fn handle_cp_drag(
    entity: Entity,
    delta: Vec2,
    is_cp1: bool,
    q_parent: &Query<&ChildOf>,
    q_computed: &Query<&ComputedNode>,
    q_row: &mut Query<&mut BezierControl>,
) {
    // cp -> canvas -> row
    let Ok(canvas_of) = q_parent.get(entity) else {
        return;
    };
    let canvas = canvas_of.parent();
    let Ok(canvas_node) = q_computed.get(canvas) else {
        return;
    };
    let w = canvas_node.size().x;
    let h = canvas_node.size().y;
    if w < 1.0 || h < 1.0 {
        return;
    }

    let Ok(row_of) = q_parent.get(canvas) else {
        return;
    };
    let row = row_of.parent();
    let Ok(mut ctrl) = q_row.get_mut(row) else {
        return;
    };

    let dx = delta.x as f64 / w as f64;
    let dy = -(delta.y as f64 / h as f64);

    if is_cp1 {
        ctrl.x1 = (ctrl.x1 + dx).clamp(0.0, 1.0);
        ctrl.y1 = (ctrl.y1 + dy).clamp(-0.5, 1.5);
    } else {
        ctrl.x2 = (ctrl.x2 + dx).clamp(0.0, 1.0);
        ctrl.y2 = (ctrl.y2 + dy).clamp(-0.5, 1.5);
    }
}

fn on_preset_cycle(
    ev: On<bevy_ui_widgets::Activate>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<(&mut BezierControl, &mut BezierPresetIndex)>,
) {
    // btn -> header -> row
    let Some(header) = q_parent.get(ev.entity).ok().map(|c| c.parent()) else {
        return;
    };
    let Some(row) = q_parent.get(header).ok().map(|c| c.parent()) else {
        return;
    };
    let Ok((mut ctrl, mut idx)) = q_row.get_mut(row) else {
        return;
    };

    idx.0 = (idx.0 + 1) % PRESETS.len();
    let preset = &PRESETS[idx.0];
    ctrl.x1 = preset.1[0];
    ctrl.y1 = preset.1[1];
    ctrl.x2 = preset.1[2];
    ctrl.y2 = preset.1[3];
}

// ══════════════════════════════════════════════════════════════════════
// Curve math
// ══════════════════════════════════════════════════════════════════════

fn cubic_bezier(t: f64, p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
}

fn find_preset_name(ctrl: &BezierControl) -> Option<&'static str> {
    for (name, vals) in PRESETS {
        if (ctrl.x1 - vals[0]).abs() < 0.01
            && (ctrl.y1 - vals[1]).abs() < 0.01
            && (ctrl.x2 - vals[2]).abs() < 0.01
            && (ctrl.y2 - vals[3]).abs() < 0.01
        {
            return Some(name);
        }
    }
    None
}

fn to_px(pct: f64) -> String {
    format!("{:.1}%", (pct * 100.0).clamp(0.0, 100.0))
}

// ══════════════════════════════════════════════════════════════════════
// Display system
// ══════════════════════════════════════════════════════════════════════

fn update_bezier_display(
    q: Query<(Entity, &BezierControl), Changed<BezierControl>>,
    q_children: Query<&Children>,
    mut q_tex: Query<(&mut BezierTexHandle, &mut ImageNode)>,
    mut q_tex_needs_init: Query<&mut BezierTexHandle, Without<ImageNode>>,
    mut q_cp1: Query<&mut InlineStyle, (With<BezierCP1>, Without<BezierCP2>)>,
    mut q_cp2: Query<&mut InlineStyle, (With<BezierCP2>, Without<BezierCP1>)>,
    mut q_x1: Query<
        &mut Text,
        (
            With<BezierX1Text>,
            Without<BezierY1Text>,
            Without<BezierX2Text>,
            Without<BezierY2Text>,
        ),
    >,
    mut q_y1: Query<
        &mut Text,
        (
            With<BezierY1Text>,
            Without<BezierX1Text>,
            Without<BezierX2Text>,
            Without<BezierY2Text>,
        ),
    >,
    mut q_x2: Query<
        &mut Text,
        (
            With<BezierX2Text>,
            Without<BezierX1Text>,
            Without<BezierY1Text>,
            Without<BezierY2Text>,
        ),
    >,
    mut q_y2: Query<
        &mut Text,
        (
            With<BezierY2Text>,
            Without<BezierX1Text>,
            Without<BezierY1Text>,
            Without<BezierX2Text>,
        ),
    >,
    mut q_preset: Query<
        &mut Text,
        (
            With<BezierPresetText>,
            Without<BezierX1Text>,
            Without<BezierY1Text>,
            Without<BezierX2Text>,
            Without<BezierY2Text>,
        ),
    >,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    for (entity, ctrl) in &q {
        let margin = 0.06;
        let usable = 1.0 - 2.0 * margin;

        let map_x = |x: f64| -> f64 { margin + x.clamp(0.0, 1.0) * usable };
        let map_y = |y: f64| -> f64 {
            let normalized = (y + 0.5) / 2.0;
            margin + (1.0 - normalized.clamp(0.0, 1.0)) * usable
        };

        for desc in q_children.iter_descendants(entity) {
            // Initialize texture if needed (first frame)
            if let Ok(mut tex_handle) = q_tex_needs_init.get_mut(desc) {
                let mut image = create_bezier_texture();
                draw_bezier_to_texture(&mut image, ctrl);
                let handle = images.add(image);
                tex_handle.0 = handle.clone();
                commands.entity(desc).insert(ImageNode {
                    image: handle,
                    ..default()
                });
                continue;
            }

            // Update existing texture
            if let Ok((tex_handle, mut img_node)) = q_tex.get_mut(desc) {
                if let Some(image) = images.get_mut(&tex_handle.0) {
                    draw_bezier_to_texture(image, ctrl);
                }
                // Ensure the image node references the correct handle
                if img_node.image != tex_handle.0 {
                    img_node.image = tex_handle.0.clone();
                }
            }

            // CP1 position
            if let Ok(mut style) = q_cp1.get_mut(desc) {
                style.set("left", to_px(map_x(ctrl.x1)));
                style.set("top", to_px(map_y(ctrl.y1)));
            }

            // CP2 position
            if let Ok(mut style) = q_cp2.get_mut(desc) {
                style.set("left", to_px(map_x(ctrl.x2)));
                style.set("top", to_px(map_y(ctrl.y2)));
            }

            // Text fields
            if let Ok(mut text) = q_x1.get_mut(desc) {
                text.0 = format!("{:.2}", ctrl.x1);
            }
            if let Ok(mut text) = q_y1.get_mut(desc) {
                text.0 = format!("{:.2}", ctrl.y1);
            }
            if let Ok(mut text) = q_x2.get_mut(desc) {
                text.0 = format!("{:.2}", ctrl.x2);
            }
            if let Ok(mut text) = q_y2.get_mut(desc) {
                text.0 = format!("{:.2}", ctrl.y2);
            }

            // Preset name
            if let Ok(mut text) = q_preset.get_mut(desc) {
                text.0 = find_preset_name(ctrl).unwrap_or("custom").to_string();
            }
        }
    }
}

fn sync_bezier_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q: Query<(&PaneControlMeta, &BezierControl), Changed<BezierControl>>,
) {
    for (meta, ctrl) in &q {
        let value = PaneValue::Custom(CustomValueBox(Box::new(BezierValue {
            x1: ctrl.x1,
            y1: ctrl.y1,
            x2: ctrl.x2,
            y2: ctrl.y2,
        })));
        if store.get_raw(&meta.pane_title, &meta.label) != Some(&value) {
            store.set_raw(&meta.pane_title, &meta.label, value.clone());
            commands.trigger(PaneChanged {
                pane: meta.pane_title.clone(),
                field: meta.label.clone(),
                value,
            });
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Builder extension
// ══════════════════════════════════════════════════════════════════════

pub trait BezierPaneExt {
    /// Add a cubic bezier curve editor with default "ease" preset.
    fn bezier(self, label: &str) -> Self;
    /// Add a cubic bezier curve editor with specific initial values.
    fn bezier_with(self, label: &str, x1: f64, y1: f64, x2: f64, y2: f64) -> Self;
}

fn bezier_config(x1: f64, y1: f64, x2: f64, y2: f64) -> ControlConfig {
    ControlConfig::new()
        .float("x1", x1)
        .float("y1", y1)
        .float("x2", x2)
        .float("y2", y2)
}

macro_rules! impl_bezier_ext {
    ($ty:ty) => {
        impl BezierPaneExt for $ty {
            fn bezier(self, label: &str) -> Self {
                self.bezier_with(label, 0.25, 0.1, 0.25, 1.0)
            }
            fn bezier_with(self, label: &str, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
                self.custom("bezier", label, bezier_config(x1, y1, x2, y2))
            }
        }
    };
}

impl_bezier_ext!(saddle_pane::prelude::PaneBuilder);
impl_bezier_ext!(saddle_pane::builder::FolderBuilder);
