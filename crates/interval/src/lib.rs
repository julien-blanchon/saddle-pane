//! # saddle_pane_interval
//!
//! Interval (range slider) plugin for [saddle_pane](https://github.com/your/saddle_pane).
//!
//! Adds a dual-thumb range control that lets users select a `min..max`
//! sub-range within configurable bounds.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use saddle_pane::prelude::*;
//! use saddle_pane_interval::PaneIntervalPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(PanePlugin)
//!         .add_plugins(PaneIntervalPlugin)  // register the interval control
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2d);
//!     PaneBuilder::new("Debug")
//!         .interval("Range", 0.0..=100.0, 20.0..=80.0)
//!         .spawn(&mut commands);
//! }
//! ```

use std::any::Any;

use bevy::asset::embedded_asset;
use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::prelude::*;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;

use saddle_pane::controls::{PaneControlMeta, PaneValue, css_percent, pane_font, spawn_label};
use saddle_pane::events::PaneChanged;
use saddle_pane::prelude::{PaneControlPlugin, PaneControlRegistry, PaneCustomValue, PaneSystems};
use saddle_pane::registry::{ControlConfig, CustomValueBox};
use saddle_pane::store::PaneStore;

const STYLE_PATH: &str = "embedded://saddle_pane_interval/style/interval.css";

// ══════════════════════════════════════════════════════════════════════
// Public types
// ══════════════════════════════════════════════════════════════════════

/// Value produced by the interval control — a min..max range.
#[derive(Clone, Debug, PartialEq)]
pub struct IntervalValue {
    pub min: f64,
    pub max: f64,
}

impl PaneCustomValue for IntervalValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn PaneCustomValue> {
        Box::new(self.clone())
    }
    fn eq_box(&self, other: &dyn PaneCustomValue) -> bool {
        other
            .as_any()
            .downcast_ref::<IntervalValue>()
            .is_some_and(|o| o == self)
    }
}

/// Component storing the interval control state.
#[derive(Component, Clone, Debug)]
pub struct IntervalControl {
    pub min: f64,
    pub max: f64,
    pub bounds_min: f64,
    pub bounds_max: f64,
    pub step: f64,
}

impl IntervalControl {
    fn range(&self) -> f64 {
        self.bounds_max - self.bounds_min
    }

    fn min_pct(&self) -> f32 {
        if self.range().abs() < f64::EPSILON {
            return 0.0;
        }
        (((self.min - self.bounds_min) / self.range()) * 100.0) as f32
    }

    fn max_pct(&self) -> f32 {
        if self.range().abs() < f64::EPSILON {
            return 100.0;
        }
        (((self.max - self.bounds_min) / self.range()) * 100.0) as f32
    }
}

/// Bevy plugin — registers the interval control with saddle_pane.
///
/// Add this after `PanePlugin`:
/// ```rust,ignore
/// app.add_plugins(PanePlugin);
/// app.add_plugins(PaneIntervalPlugin);
/// ```
pub struct PaneIntervalPlugin;

impl Plugin for PaneIntervalPlugin {
    fn build(&self, app: &mut App) {
        // Embed CSS
        embedded_asset!(app, "style/interval.css");

        // Register in the pane control registry
        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "interval",
            build: build_interval_systems,
            spawn: spawn_interval_ui,
            default_value: interval_default_value,
        });

        // Register systems
        build_interval_systems(app);
    }
}

// ══════════════════════════════════════════════════════════════════════
// Internal components
// ══════════════════════════════════════════════════════════════════════

#[derive(Component, Clone, Debug, Default)]
struct IntervalMinThumb;

#[derive(Component, Clone, Debug, Default)]
struct IntervalMaxThumb;

#[derive(Component, Clone, Debug, Default)]
struct IntervalFill;

#[derive(Component, Clone, Debug, Default)]
struct IntervalMinText;

#[derive(Component, Clone, Debug, Default)]
struct IntervalMaxText;

#[derive(Component, Clone, Debug, Default)]
struct IntervalDragState {
    dragging: bool,
    is_min: bool,
}

// ══════════════════════════════════════════════════════════════════════
// Plugin descriptor functions
// ══════════════════════════════════════════════════════════════════════

fn build_interval_systems(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update_interval_display.in_set(PaneSystems::Display),
    );
    app.add_systems(
        PostUpdate,
        sync_interval_to_store.in_set(PaneSystems::Sync),
    );
}

fn interval_default_value(config: &ControlConfig) -> Option<PaneValue> {
    let min = config.get_float("default_min").unwrap_or(0.0);
    let max = config.get_float("default_max").unwrap_or(1.0);
    Some(PaneValue::Custom(CustomValueBox(Box::new(IntervalValue {
        min,
        max,
    }))))
}

// ══════════════════════════════════════════════════════════════════════
// Spawn
// ══════════════════════════════════════════════════════════════════════

fn spawn_interval_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let bounds_min = config.get_float("bounds_min").unwrap_or(0.0);
    let bounds_max = config.get_float("bounds_max").unwrap_or(1.0);
    let default_min = config.get_float("default_min").unwrap_or(bounds_min);
    let default_max = config.get_float("default_max").unwrap_or(bounds_max);
    let step = config.get_float("step").unwrap_or(0.01);

    let control = IntervalControl {
        min: default_min,
        max: default_max,
        bounds_min,
        bounds_max,
        step,
    };

    let min_pct = control.min_pct();
    let max_pct = control.max_pct();
    let fill_width = max_pct - min_pct;

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control,
            IntervalDragState::default(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            spawn_label(row, &meta.label);

            // Interval area
            row.spawn((Node::default(), ClassList::new("pane-interval")))
                .with_children(|area| {
                    // Min value text
                    area.spawn((Node::default(), ClassList::new("pane-interval-value")))
                        .with_children(|v| {
                            v.spawn((
                                Text::new(format!("{default_min:.2}")),
                                pane_font(9.0),
                                ClassList::new("pane-interval-value-text"),
                                IntervalMinText,
                            ));
                        });

                    // Track
                    area.spawn((
                        Node::default(),
                        Interaction::default(),
                        ClassList::new("pane-interval-track"),
                        InlineStyle::from_iter([
                            ("--interval-min-pos", css_percent(min_pct)),
                            ("--interval-max-pos", css_percent(max_pct)),
                            ("--interval-fill-left", css_percent(min_pct)),
                            ("--interval-fill-width", css_percent(fill_width)),
                        ]),
                    ))
                    .with_children(|track| {
                        // Fill between thumbs
                        track.spawn((
                            Node::default(),
                            ClassList::new("pane-interval-fill"),
                            IntervalFill,
                        ));

                        // Min thumb
                        track.spawn((
                            Node::default(),
                            Interaction::default(),
                            ClassList::new("pane-interval-thumb-min"),
                            IntervalMinThumb,
                            bevy_ui_widgets::observe(on_min_drag_start),
                            bevy_ui_widgets::observe(on_min_drag),
                            bevy_ui_widgets::observe(on_min_drag_end),
                        ));

                        // Max thumb
                        track.spawn((
                            Node::default(),
                            Interaction::default(),
                            ClassList::new("pane-interval-thumb-max"),
                            IntervalMaxThumb,
                            bevy_ui_widgets::observe(on_max_drag_start),
                            bevy_ui_widgets::observe(on_max_drag),
                            bevy_ui_widgets::observe(on_max_drag_end),
                        ));
                    });

                    // Max value text
                    area.spawn((Node::default(), ClassList::new("pane-interval-value")))
                        .with_children(|v| {
                            v.spawn((
                                Text::new(format!("{default_max:.2}")),
                                pane_font(9.0),
                                ClassList::new("pane-interval-value-text"),
                                IntervalMaxText,
                            ));
                        });
                });
        });

    row_entity
}

// ══════════════════════════════════════════════════════════════════════
// Drag handlers
// ══════════════════════════════════════════════════════════════════════

fn on_min_drag_start(
    ev: On<Pointer<DragStart>>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut IntervalDragState>,
) {
    if let Some(row) = walk_to_row(ev.entity, &q_parent) {
        if let Ok(mut drag) = q_row.get_mut(row) {
            drag.dragging = true;
            drag.is_min = true;
        }
    }
}

fn on_max_drag_start(
    ev: On<Pointer<DragStart>>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut IntervalDragState>,
) {
    if let Some(row) = walk_to_row(ev.entity, &q_parent) {
        if let Ok(mut drag) = q_row.get_mut(row) {
            drag.dragging = true;
            drag.is_min = false;
        }
    }
}

fn on_min_drag(
    ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    q_track: Query<&ComputedNode>,
    mut q_row: Query<(&mut IntervalControl, &IntervalDragState)>,
) {
    handle_drag(ev.entity, ev.event().delta.x, true, &q_parent, &q_track, &mut q_row);
}

fn on_max_drag(
    ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    q_track: Query<&ComputedNode>,
    mut q_row: Query<(&mut IntervalControl, &IntervalDragState)>,
) {
    handle_drag(ev.entity, ev.event().delta.x, false, &q_parent, &q_track, &mut q_row);
}

fn on_min_drag_end(
    ev: On<Pointer<DragEnd>>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut IntervalDragState>,
) {
    if let Some(row) = walk_to_row(ev.entity, &q_parent) {
        if let Ok(mut drag) = q_row.get_mut(row) {
            drag.dragging = false;
        }
    }
}

fn on_max_drag_end(
    ev: On<Pointer<DragEnd>>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut IntervalDragState>,
) {
    if let Some(row) = walk_to_row(ev.entity, &q_parent) {
        if let Ok(mut drag) = q_row.get_mut(row) {
            drag.dragging = false;
        }
    }
}

/// Walk up: thumb → track → area → row
fn walk_to_row(entity: Entity, q_parent: &Query<&ChildOf>) -> Option<Entity> {
    let track = q_parent.get(entity).ok()?.parent();
    let area = q_parent.get(track).ok()?.parent();
    let row = q_parent.get(area).ok()?.parent();
    Some(row)
}

fn handle_drag(
    thumb_entity: Entity,
    delta_x: f32,
    is_min: bool,
    q_parent: &Query<&ChildOf>,
    q_track: &Query<&ComputedNode>,
    q_row: &mut Query<(&mut IntervalControl, &IntervalDragState)>,
) {
    let Ok(track_of) = q_parent.get(thumb_entity) else { return };
    let track_entity = track_of.parent();
    let Ok(track_node) = q_track.get(track_entity) else { return };
    let track_width = track_node.size().x;
    if track_width < 1.0 { return; }

    let Some(row) = walk_to_row(thumb_entity, q_parent) else { return };
    let Ok((mut ctrl, _)) = q_row.get_mut(row) else { return };

    let range = ctrl.bounds_max - ctrl.bounds_min;
    let delta_val = (delta_x as f64 / track_width as f64) * range;
    let step = ctrl.step;

    if is_min {
        ctrl.min = snap(ctrl.min + delta_val, step).clamp(ctrl.bounds_min, ctrl.max);
    } else {
        ctrl.max = snap(ctrl.max + delta_val, step).clamp(ctrl.min, ctrl.bounds_max);
    }
}

fn snap(value: f64, step: f64) -> f64 {
    if step <= 0.0 { return value; }
    (value / step).round() * step
}

// ══════════════════════════════════════════════════════════════════════
// Systems
// ══════════════════════════════════════════════════════════════════════

fn update_interval_display(
    q: Query<(Entity, &IntervalControl), Changed<IntervalControl>>,
    q_children: Query<&Children>,
    mut q_style: Query<&mut InlineStyle>,
    mut q_min_text: Query<&mut Text, (With<IntervalMinText>, Without<IntervalMaxText>)>,
    mut q_max_text: Query<&mut Text, (With<IntervalMaxText>, Without<IntervalMinText>)>,
) {
    for (entity, ctrl) in &q {
        let min_pct_val = ctrl.min_pct();
        let max_pct_val = ctrl.max_pct();
        let min_pct = css_percent(min_pct_val);
        let max_pct = css_percent(max_pct_val);
        let fill_left = css_percent(min_pct_val);
        let fill_width = css_percent(max_pct_val - min_pct_val);

        for desc in q_children.iter_descendants(entity) {
            if let Ok(mut style) = q_style.get_mut(desc) {
                if style.get("--interval-min-pos").is_some() {
                    style.set("--interval-min-pos", min_pct.clone());
                    style.set("--interval-max-pos", max_pct.clone());
                    style.set("--interval-fill-left", fill_left.clone());
                    style.set("--interval-fill-width", fill_width.clone());
                }
            }
        }

        for desc in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_min_text.get_mut(desc) {
                text.0 = format!("{:.2}", ctrl.min);
            }
            if let Ok(mut text) = q_max_text.get_mut(desc) {
                text.0 = format!("{:.2}", ctrl.max);
            }
        }
    }
}

fn sync_interval_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q: Query<(&PaneControlMeta, &IntervalControl), Changed<IntervalControl>>,
) {
    for (meta, ctrl) in &q {
        let value = PaneValue::Custom(CustomValueBox(Box::new(IntervalValue {
            min: ctrl.min,
            max: ctrl.max,
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
