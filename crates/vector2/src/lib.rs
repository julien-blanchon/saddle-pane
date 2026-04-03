//! # saddle_pane_vector2
//!
//! Vector2 (2D point) plugin for [saddle_pane](https://github.com/your/saddle_pane).
//!
//! Provides a joystick-style pad + X/Y number fields for editing a 2D vector.

use std::any::Any;

use bevy::asset::embedded_asset;
use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::prelude::*;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;

use saddle_pane::controls::{PaneControlMeta, PaneValue, css_percent, pane_font, spawn_label};
use saddle_pane::icons::spawn_pane_icon;
use saddle_pane::events::PaneChanged;
use saddle_pane::prelude::{PaneControlPlugin, PaneControlRegistry, PaneCustomValue, PaneSystems};
use saddle_pane::registry::{ControlConfig, CustomValueBox};
use saddle_pane::store::PaneStore;

const STYLE_PATH: &str = "embedded://saddle_pane_vector2/style/vector2.css";

// ══════════════════════════════════════════════════════════════════════
// Public types
// ══════════════════════════════════════════════════════════════════════

/// Value produced by the vector2 control.
#[derive(Clone, Debug, PartialEq)]
pub struct Vector2Value {
    pub x: f64,
    pub y: f64,
}

impl PaneCustomValue for Vector2Value {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn PaneCustomValue> {
        Box::new(self.clone())
    }
    fn eq_box(&self, other: &dyn PaneCustomValue) -> bool {
        other
            .as_any()
            .downcast_ref::<Vector2Value>()
            .is_some_and(|o| o == self)
    }
}

/// Component storing the vector2 control state.
#[derive(Component, Clone, Debug)]
pub struct Vector2Control {
    pub x: f64,
    pub y: f64,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub step: f64,
    pub joystick: bool,
    pub invert_y: bool,
    pub pad_open: bool,
}

impl Vector2Control {
    fn x_pct(&self) -> f32 {
        let range = self.max_x - self.min_x;
        if range.abs() < f64::EPSILON {
            return 50.0;
        }
        (((self.x - self.min_x) / range) * 100.0) as f32
    }

    fn y_pct(&self) -> f32 {
        let range = self.max_y - self.min_y;
        if range.abs() < f64::EPSILON {
            return 50.0;
        }
        let pct = ((self.y - self.min_y) / range) * 100.0;
        // Invert for screen coords (top=0)
        (100.0 - pct) as f32
    }
}

// ══════════════════════════════════════════════════════════════════════
// Internal markers
// ══════════════════════════════════════════════════════════════════════

#[derive(Component, Clone, Debug, Default)]
struct Vec2JoystickBtn;

#[derive(Component, Clone, Debug, Default)]
struct Vec2PadPopup;

#[derive(Component, Clone, Debug, Default)]
struct Vec2Pad;

#[derive(Component, Clone, Debug, Default)]
struct Vec2Dot;

#[derive(Component, Clone, Debug, Default)]
struct Vec2XText;

#[derive(Component, Clone, Debug, Default)]
struct Vec2YText;

#[derive(Component, Clone, Debug, Default)]
struct Vec2DragState {
    dragging: bool,
}

// ══════════════════════════════════════════════════════════════════════
// Plugin
// ══════════════════════════════════════════════════════════════════════

pub struct PaneVector2Plugin;

impl Plugin for PaneVector2Plugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "style/vector2.css");

        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "vector2",
            build: build_systems,
            spawn: spawn_vector2_ui,
            default_value: vector2_default_value,
        });

        build_systems(app);
    }
}

fn build_systems(app: &mut App) {
    app.add_systems(
        Update,
        (close_pad_on_escape, close_pad_on_click_outside),
    );
    app.add_systems(
        PostUpdate,
        update_vector2_display.in_set(PaneSystems::Display),
    );
    app.add_systems(
        PostUpdate,
        sync_vector2_to_store.in_set(PaneSystems::Sync),
    );
}

fn vector2_default_value(config: &ControlConfig) -> Option<PaneValue> {
    let x = config.get_float("default_x").unwrap_or(0.0);
    let y = config.get_float("default_y").unwrap_or(0.0);
    Some(PaneValue::Custom(CustomValueBox(Box::new(Vector2Value {
        x,
        y,
    }))))
}

// ══════════════════════════════════════════════════════════════════════
// Spawn
// ══════════════════════════════════════════════════════════════════════

fn spawn_vector2_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let default_x = config.get_float("default_x").unwrap_or(0.0);
    let default_y = config.get_float("default_y").unwrap_or(0.0);
    let min_x = config.get_float("min_x").unwrap_or(-100.0);
    let max_x = config.get_float("max_x").unwrap_or(100.0);
    let min_y = config.get_float("min_y").unwrap_or(-100.0);
    let max_y = config.get_float("max_y").unwrap_or(100.0);
    let step = config.get_float("step").unwrap_or(0.1);
    let joystick = config.get_bool("joystick").unwrap_or(true);
    let invert_y = config.get_bool("invert_y").unwrap_or(false);

    let ctrl = Vector2Control {
        x: default_x,
        y: default_y,
        min_x,
        max_x,
        min_y,
        max_y,
        step,
        joystick,
        invert_y,
        pad_open: false,
    };

    let x_pct = ctrl.x_pct();
    let y_pct = ctrl.y_pct();

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            ctrl,
            Vec2DragState::default(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            spawn_label(row, &meta.label);

            // Main area
            row.spawn((Node::default(), ClassList::new("pane-vec2-area")))
                .with_children(|area| {
                    if joystick {
                        // Joystick trigger button
                        area.spawn((
                            Node::default(),
                            Interaction::default(),
                            bevy_ui_widgets::Button,
                            ClassList::new("pane-vec2-joystick-btn"),
                            Vec2JoystickBtn,
                            bevy_ui_widgets::observe(on_joystick_toggle),
                        ))
                        .with_children(|btn| {
                            spawn_pane_icon(btn, saddle_pane::icons::ICON_CROSSHAIR, 14.0);

                            // Popup pad (initially hidden)
                            btn.spawn((
                                Node::default(),
                                ClassList::new("pane-vec2-pad-popup"),
                                Vec2PadPopup,
                            ))
                            .with_children(|popup| {
                                popup
                                    .spawn((
                                        Node::default(),
                                        Interaction::default(),
                                        ClassList::new("pane-vec2-pad"),
                                        Vec2Pad,
                                        bevy_ui_widgets::observe(on_pad_drag_start),
                                        bevy_ui_widgets::observe(on_pad_drag),
                                        bevy_ui_widgets::observe(on_pad_drag_end),
                                    ))
                                    .with_children(|pad| {
                                        // Cross-hairs
                                        pad.spawn((
                                            Node::default(),
                                            ClassList::new("pane-vec2-pad-hline"),
                                        ));
                                        pad.spawn((
                                            Node::default(),
                                            ClassList::new("pane-vec2-pad-vline"),
                                        ));

                                        // Dot
                                        pad.spawn((
                                            Node::default(),
                                            ClassList::new("pane-vec2-dot"),
                                            Vec2Dot,
                                            InlineStyle::from_iter([
                                                ("left", css_percent(x_pct)),
                                                ("top", css_percent(y_pct)),
                                            ]),
                                        ));
                                    });
                            });
                        });
                    }

                    // Number fields
                    area.spawn((Node::default(), ClassList::new("pane-vec2-fields")))
                        .with_children(|fields| {
                            // X field
                            fields
                                .spawn((
                                    Node::default(),
                                    ClassList::new("pane-vec2-field-group"),
                                ))
                                .with_children(|g| {
                                    g.spawn((
                                        Text::new("X"),
                                        pane_font(10.0),
                                        ClassList::new("pane-vec2-field-label"),
                                    ));
                                    g.spawn((
                                        Node::default(),
                                        ClassList::new("pane-vec2-field-value"),
                                    ))
                                    .with_children(|v| {
                                        v.spawn((
                                            Text::new(format!("{default_x:.1}")),
                                            pane_font(10.0),
                                            ClassList::new("pane-vec2-field-text"),
                                            Vec2XText,
                                        ));
                                    });
                                });

                            // Y field
                            fields
                                .spawn((
                                    Node::default(),
                                    ClassList::new("pane-vec2-field-group"),
                                ))
                                .with_children(|g| {
                                    g.spawn((
                                        Text::new("Y"),
                                        pane_font(10.0),
                                        ClassList::new("pane-vec2-field-label"),
                                    ));
                                    g.spawn((
                                        Node::default(),
                                        ClassList::new("pane-vec2-field-value"),
                                    ))
                                    .with_children(|v| {
                                        v.spawn((
                                            Text::new(format!("{default_y:.1}")),
                                            pane_font(10.0),
                                            ClassList::new("pane-vec2-field-text"),
                                            Vec2YText,
                                        ));
                                    });
                                });
                        });
                });
        });

    row_entity
}

// ══════════════════════════════════════════════════════════════════════
// Interaction handlers
// ══════════════════════════════════════════════════════════════════════

fn on_joystick_toggle(
    ev: On<bevy_ui_widgets::Activate>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut Vector2Control>,
) {
    // btn -> area -> row
    let Some(area) = q_parent.get(ev.entity).ok().map(|c| c.parent()) else {
        return;
    };
    let Some(row) = q_parent.get(area).ok().map(|c| c.parent()) else {
        return;
    };
    if let Ok(mut ctrl) = q_row.get_mut(row) {
        ctrl.pad_open = !ctrl.pad_open;
    }
}

fn on_pad_drag_start(
    ev: On<Pointer<DragStart>>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut Vec2DragState>,
) {
    if let Some(row) = walk_pad_to_row(ev.entity, &q_parent) {
        if let Ok(mut drag) = q_row.get_mut(row) {
            drag.dragging = true;
        }
    }
}

fn on_pad_drag(
    ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    q_computed: Query<&ComputedNode>,
    mut q_row: Query<(&mut Vector2Control, &Vec2DragState)>,
) {
    let Some(row) = walk_pad_to_row(ev.entity, &q_parent) else {
        return;
    };
    let Ok((mut ctrl, _)) = q_row.get_mut(row) else {
        return;
    };
    let Ok(pad_node) = q_computed.get(ev.entity) else {
        return;
    };

    let pad_w = pad_node.size().x;
    let pad_h = pad_node.size().y;
    if pad_w < 1.0 || pad_h < 1.0 {
        return;
    }

    let dx = ev.event().delta.x as f64 / pad_w as f64;
    let dy = ev.event().delta.y as f64 / pad_h as f64;

    let range_x = ctrl.max_x - ctrl.min_x;
    let range_y = ctrl.max_y - ctrl.min_y;
    let step = ctrl.step;
    let invert_y = ctrl.invert_y;

    ctrl.x = snap(ctrl.x + dx * range_x, step).clamp(ctrl.min_x, ctrl.max_x);
    let y_delta = if invert_y { dy } else { -dy };
    ctrl.y = snap(ctrl.y + y_delta * range_y, step).clamp(ctrl.min_y, ctrl.max_y);
}

fn on_pad_drag_end(
    ev: On<Pointer<DragEnd>>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut Vec2DragState>,
) {
    if let Some(row) = walk_pad_to_row(ev.entity, &q_parent) {
        if let Ok(mut drag) = q_row.get_mut(row) {
            drag.dragging = false;
        }
    }
}

/// Walk up: pad -> popup -> btn -> area -> row
fn walk_pad_to_row(entity: Entity, q_parent: &Query<&ChildOf>) -> Option<Entity> {
    let popup = q_parent.get(entity).ok()?.parent();
    let btn = q_parent.get(popup).ok()?.parent();
    let area = q_parent.get(btn).ok()?.parent();
    let row = q_parent.get(area).ok()?.parent();
    Some(row)
}

fn snap(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return value;
    }
    (value / step).round() * step
}

// ══════════════════════════════════════════════════════════════════════
// Auto-close systems
// ══════════════════════════════════════════════════════════════════════

fn close_pad_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Vector2Control>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for mut ctrl in &mut q {
            if ctrl.pad_open {
                ctrl.pad_open = false;
            }
        }
    }
}

fn close_pad_on_click_outside(
    mouse: Res<ButtonInput<MouseButton>>,
    mut q: Query<(Entity, &mut Vector2Control)>,
    q_popup: Query<Entity, With<Vec2PadPopup>>,
    q_btn: Query<Entity, With<Vec2JoystickBtn>>,
    q_children: Query<&Children>,
    q_interaction: Query<&Interaction>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    for (row_entity, mut ctrl) in &mut q {
        if !ctrl.pad_open {
            continue;
        }

        // Check if the click hit the popup, any popup descendant, or the joystick button
        let mut hit = false;

        for desc in q_children.iter_descendants(row_entity) {
            // Check popup and its descendants
            if q_popup.get(desc).is_ok() || q_btn.get(desc).is_ok() {
                if q_interaction
                    .get(desc)
                    .is_ok_and(|i| *i == Interaction::Pressed)
                {
                    hit = true;
                    break;
                }
                // Also check descendants of popup/btn
                for inner in q_children.iter_descendants(desc) {
                    if q_interaction
                        .get(inner)
                        .is_ok_and(|i| *i == Interaction::Pressed)
                    {
                        hit = true;
                        break;
                    }
                }
                if hit {
                    break;
                }
            }
        }

        if !hit {
            ctrl.pad_open = false;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Systems
// ══════════════════════════════════════════════════════════════════════

fn update_vector2_display(
    q: Query<(Entity, &Vector2Control), Changed<Vector2Control>>,
    q_children: Query<&Children>,
    mut q_x_text: Query<&mut Text, (With<Vec2XText>, Without<Vec2YText>)>,
    mut q_y_text: Query<&mut Text, (With<Vec2YText>, Without<Vec2XText>)>,
    mut q_dot_style: Query<&mut InlineStyle, With<Vec2Dot>>,
    mut q_popup_class: Query<&mut ClassList, With<Vec2PadPopup>>,
    mut q_btn_class: Query<&mut ClassList, (With<Vec2JoystickBtn>, Without<Vec2PadPopup>)>,
) {
    for (entity, ctrl) in &q {
        let x_pct = ctrl.x_pct();
        let y_pct = ctrl.y_pct();

        for desc in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_x_text.get_mut(desc) {
                text.0 = format!("{:.1}", ctrl.x);
            }
            if let Ok(mut text) = q_y_text.get_mut(desc) {
                text.0 = format!("{:.1}", ctrl.y);
            }
            if let Ok(mut style) = q_dot_style.get_mut(desc) {
                style.set("left", css_percent(x_pct));
                style.set("top", css_percent(y_pct));
            }
            if let Ok(mut class) = q_popup_class.get_mut(desc) {
                if ctrl.pad_open {
                    *class = ClassList::new("pane-vec2-pad-popup is-open");
                } else {
                    *class = ClassList::new("pane-vec2-pad-popup");
                }
            }
            if let Ok(mut class) = q_btn_class.get_mut(desc) {
                if ctrl.pad_open {
                    *class = ClassList::new("pane-vec2-joystick-btn is-active");
                } else {
                    *class = ClassList::new("pane-vec2-joystick-btn");
                }
            }
        }
    }
}

fn sync_vector2_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q: Query<(&PaneControlMeta, &Vector2Control), Changed<Vector2Control>>,
) {
    for (meta, ctrl) in &q {
        let value = PaneValue::Custom(CustomValueBox(Box::new(Vector2Value {
            x: ctrl.x,
            y: ctrl.y,
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

pub trait Vector2PaneExt {
    /// Add a vector2 control with joystick pad and number fields.
    fn vector2(self, label: &str, default: (f64, f64)) -> Self;
    /// Add a vector2 control with custom bounds.
    fn vector2_bounded(
        self,
        label: &str,
        x_range: std::ops::RangeInclusive<f64>,
        y_range: std::ops::RangeInclusive<f64>,
        default: (f64, f64),
    ) -> Self;
}

fn vec2_config(default: (f64, f64), min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> ControlConfig {
    ControlConfig::new()
        .float("default_x", default.0)
        .float("default_y", default.1)
        .float("min_x", min_x)
        .float("max_x", max_x)
        .float("min_y", min_y)
        .float("max_y", max_y)
        .float("step", 0.1)
        .bool("joystick", true)
}

macro_rules! impl_vec2_ext {
    ($ty:ty) => {
        impl Vector2PaneExt for $ty {
            fn vector2(self, label: &str, default: (f64, f64)) -> Self {
                self.custom("vector2", label, vec2_config(default, -100.0, 100.0, -100.0, 100.0))
            }
            fn vector2_bounded(
                self,
                label: &str,
                x_range: std::ops::RangeInclusive<f64>,
                y_range: std::ops::RangeInclusive<f64>,
                default: (f64, f64),
            ) -> Self {
                self.custom("vector2", label, vec2_config(
                    default,
                    *x_range.start(), *x_range.end(),
                    *y_range.start(), *y_range.end(),
                ))
            }
        }
    };
}

impl_vec2_ext!(saddle_pane::prelude::PaneBuilder);
impl_vec2_ext!(saddle_pane::builder::FolderBuilder);
