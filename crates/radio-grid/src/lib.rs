//! # saddle_pane_radio_grid
//!
//! Radio/checkbox grid plugin for [saddle_pane](https://github.com/your/saddle_pane).
//!
//! A grid of toggle buttons with configurable selection behavior:
//! - **Radio** (`max_selected = 1`): only one can be selected at a time
//! - **Multi-select** (`max_selected = N`): up to N can be selected
//! - **Unlimited** (`max_selected = 0`): any number can be selected

use std::any::Any;

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;

use saddle_pane::controls::{PaneControlMeta, PaneValue, pane_font, spawn_label};
use saddle_pane::events::PaneChanged;
use saddle_pane::prelude::{PaneControlPlugin, PaneControlRegistry, PaneCustomValue, PaneSystems};
use saddle_pane::registry::{ControlConfig, CustomValueBox};
use saddle_pane::store::PaneStore;

const STYLE_PATH: &str = "embedded://saddle_pane_radio_grid/style/radio_grid.css";

// ══════════════════════════════════════════════════════════════════════
// Public types
// ══════════════════════════════════════════════════════════════════════

/// Value produced by the radio grid — set of selected indices.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioGridValue {
    pub selected: Vec<usize>,
    pub labels: Vec<String>,
}

impl RadioGridValue {
    /// Get the selected label(s).
    pub fn selected_labels(&self) -> Vec<&str> {
        self.selected
            .iter()
            .filter_map(|&i| self.labels.get(i).map(|s| s.as_str()))
            .collect()
    }
}

impl PaneCustomValue for RadioGridValue {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn PaneCustomValue> {
        Box::new(self.clone())
    }
    fn eq_box(&self, other: &dyn PaneCustomValue) -> bool {
        other
            .as_any()
            .downcast_ref::<RadioGridValue>()
            .is_some_and(|o| o == self)
    }
}

/// Component storing the radio grid state.
#[derive(Component, Clone, Debug)]
pub struct RadioGridControl {
    pub labels: Vec<String>,
    pub selected: Vec<usize>,
    /// 0 = unlimited, 1 = radio, N = multi-select with limit
    pub max_selected: usize,
}

/// Marker for individual grid item entities, storing their index.
#[derive(Component, Clone, Debug)]
struct RadioGridItem {
    index: usize,
}

// ══════════════════════════════════════════════════════════════════════
// Plugin
// ══════════════════════════════════════════════════════════════════════

pub struct PaneRadioGridPlugin;

impl Plugin for PaneRadioGridPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "style/radio_grid.css");

        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "radio_grid",
            build: build_systems,
            spawn: spawn_radio_grid_ui,
            default_value: radio_grid_default_value,
        });

        build_systems(app);
    }
}

fn build_systems(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (update_radio_grid_display, sync_radio_grid_to_store)
            .chain()
            .in_set(PaneSystems::Display),
    );
}

fn radio_grid_default_value(config: &ControlConfig) -> Option<PaneValue> {
    let labels = config
        .get_string_list("labels")
        .map(|s| s.to_vec())
        .unwrap_or_default();
    let default_selected = config
        .get_float_list("default_selected")
        .map(|v| v.iter().map(|f| *f as usize).collect())
        .unwrap_or_default();
    Some(PaneValue::Custom(CustomValueBox(Box::new(
        RadioGridValue {
            selected: default_selected,
            labels,
        },
    ))))
}

// ══════════════════════════════════════════════════════════════════════
// Spawn
// ══════════════════════════════════════════════════════════════════════

fn spawn_radio_grid_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let labels: Vec<String> = config
        .get_string_list("labels")
        .map(|s| s.to_vec())
        .unwrap_or_default();
    let max_selected = config.get_int("max_selected").unwrap_or(1) as usize;
    let default_selected: Vec<usize> = config
        .get_float_list("default_selected")
        .map(|v| v.iter().map(|f| *f as usize).collect())
        .unwrap_or_default();

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(4.0), Val::Px(6.0)),
                column_gap: Val::Px(6.0),
                min_width: Val::ZERO,
                ..default()
            },
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            RadioGridControl {
                labels: labels.clone(),
                selected: default_selected.clone(),
                max_selected,
            },
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            spawn_label(row, &meta.label);

            // Grid container
            row.spawn((Node::default(), ClassList::new("pane-radio-grid")))
                .with_children(|grid| {
                    for (i, label) in labels.iter().enumerate() {
                        let is_selected = default_selected.contains(&i);
                        let mut classes = "pane-radio-grid-item".to_string();
                        if is_selected {
                            classes.push_str(" is-selected");
                        }

                        grid.spawn((
                            Node::default(),
                            Interaction::default(),
                            bevy_ui_widgets::Button,
                            ClassList::new(&classes),
                            RadioGridItem { index: i },
                            bevy_ui_widgets::observe(on_grid_item_click),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(label),
                                pane_font(10.0),
                                ClassList::new("pane-radio-grid-item-text"),
                            ));
                        });
                    }
                });
        });

    row_entity
}

// ══════════════════════════════════════════════════════════════════════
// Interaction
// ══════════════════════════════════════════════════════════════════════

fn on_grid_item_click(
    ev: On<bevy_ui_widgets::Activate>,
    q_item: Query<&RadioGridItem>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut RadioGridControl>,
) {
    let Ok(item) = q_item.get(ev.entity) else {
        return;
    };
    let idx = item.index;

    // Walk up: item -> grid -> row
    let Some(grid) = q_parent.get(ev.entity).ok().map(|c| c.parent()) else {
        return;
    };
    let Some(row) = q_parent.get(grid).ok().map(|c| c.parent()) else {
        return;
    };
    let Ok(mut ctrl) = q_row.get_mut(row) else {
        return;
    };

    if ctrl.selected.contains(&idx) {
        // Deselect (unless radio mode and it's the only one)
        if ctrl.max_selected != 1 || ctrl.selected.len() > 1 {
            ctrl.selected.retain(|&i| i != idx);
        }
    } else {
        // Select
        if ctrl.max_selected == 1 {
            // Radio: replace
            ctrl.selected.clear();
            ctrl.selected.push(idx);
        } else if ctrl.max_selected == 0 || ctrl.selected.len() < ctrl.max_selected {
            // Multi or unlimited: add
            ctrl.selected.push(idx);
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Systems
// ══════════════════════════════════════════════════════════════════════

fn update_radio_grid_display(
    q: Query<(Entity, &RadioGridControl), Changed<RadioGridControl>>,
    q_children: Query<&Children>,
    q_item: Query<&RadioGridItem>,
    mut q_class: Query<&mut ClassList>,
) {
    for (entity, ctrl) in &q {
        for desc in q_children.iter_descendants(entity) {
            if let Ok(item) = q_item.get(desc) {
                if let Ok(mut class) = q_class.get_mut(desc) {
                    if ctrl.selected.contains(&item.index) {
                        *class = ClassList::new("pane-radio-grid-item is-selected");
                    } else {
                        *class = ClassList::new("pane-radio-grid-item");
                    }
                }
            }
        }
    }
}

fn sync_radio_grid_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q: Query<(&PaneControlMeta, &RadioGridControl), Changed<RadioGridControl>>,
) {
    for (meta, ctrl) in &q {
        let value = PaneValue::Custom(CustomValueBox(Box::new(RadioGridValue {
            selected: ctrl.selected.clone(),
            labels: ctrl.labels.clone(),
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

fn radio_config(options: &[&str], max_selected: usize, defaults: &[usize]) -> ControlConfig {
    ControlConfig::new()
        .string_list("labels", options.iter().map(|s| s.to_string()).collect())
        .int("max_selected", max_selected as i64)
        .float_list(
            "default_selected",
            defaults.iter().map(|&i| i as f64).collect(),
        )
}

pub trait RadioGridPaneExt {
    /// Add a radio grid (single selection).
    fn radio_grid(self, label: &str, options: &[&str], default: usize) -> Self;
    /// Add a checkbox grid (unlimited selection).
    fn checkbox_grid(self, label: &str, options: &[&str], defaults: &[usize]) -> Self;
    /// Add a multi-select grid with a selection limit.
    fn multi_grid(
        self,
        label: &str,
        options: &[&str],
        max_selected: usize,
        defaults: &[usize],
    ) -> Self;
}

macro_rules! impl_radio_grid_ext {
    ($ty:ty) => {
        impl RadioGridPaneExt for $ty {
            fn radio_grid(self, label: &str, options: &[&str], default: usize) -> Self {
                self.custom("radio_grid", label, radio_config(options, 1, &[default]))
            }
            fn checkbox_grid(self, label: &str, options: &[&str], defaults: &[usize]) -> Self {
                self.custom("radio_grid", label, radio_config(options, 0, defaults))
            }
            fn multi_grid(
                self,
                label: &str,
                options: &[&str],
                max_selected: usize,
                defaults: &[usize],
            ) -> Self {
                self.custom(
                    "radio_grid",
                    label,
                    radio_config(options, max_selected, defaults),
                )
            }
        }
    };
}

impl_radio_grid_ext!(saddle_pane::prelude::PaneBuilder);
impl_radio_grid_ext!(saddle_pane::builder::FolderBuilder);
