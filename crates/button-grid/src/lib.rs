//! # saddle_pane_button_grid
//!
//! Button grid plugin for [saddle_pane](https://github.com/your/saddle_pane).
//!
//! Adds a grid of buttons that fire `PaneButtonPressed` events when clicked.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;

use saddle_pane::controls::{PaneControlMeta, pane_font, spawn_label};
use saddle_pane::events::PaneButtonPressed;
use saddle_pane::prelude::{PaneControlPlugin, PaneControlRegistry};
use saddle_pane::registry::ControlConfig;

const STYLE_PATH: &str = "embedded://saddle_pane_button_grid/style/button_grid.css";

// ══════════════════════════════════════════════════════════════════════
// Public types
// ══════════════════════════════════════════════════════════════════════

/// Component storing the button grid state.
#[derive(Component, Clone, Debug)]
pub struct ButtonGridControl {
    pub labels: Vec<String>,
    pub columns: usize,
}

/// Marker for individual grid button entities.
#[derive(Component, Clone, Debug)]
#[allow(dead_code)]
struct GridButton {
    index: usize,
    label: String,
}

// ══════════════════════════════════════════════════════════════════════
// Plugin
// ══════════════════════════════════════════════════════════════════════

pub struct PaneButtonGridPlugin;

impl Plugin for PaneButtonGridPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "style/button_grid.css");

        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "button_grid",
            build: build_systems,
            spawn: spawn_button_grid_ui,
            default_value: |_| None, // buttons don't produce values
        });

        build_systems(app);
    }
}

fn build_systems(_app: &mut App) {
    // No sync/display systems needed — buttons fire events directly via observers
}

// ══════════════════════════════════════════════════════════════════════
// Spawn
// ══════════════════════════════════════════════════════════════════════

fn spawn_button_grid_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let labels: Vec<String> = config
        .get_string_list("labels")
        .map(|s| s.to_vec())
        .unwrap_or_default();
    let columns = config.get_int("columns").unwrap_or(3) as usize;

    let meta_clone = meta.clone();
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
            ButtonGridControl {
                labels: labels.clone(),
                columns,
            },
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            spawn_label(row, &meta_clone.label);

            // Grid container
            row.spawn((Node::default(), ClassList::new("pane-button-grid")))
                .with_children(|grid| {
                    for (i, label) in labels.iter().enumerate() {
                        let meta_for_btn = meta_clone.clone();
                        let btn_label = label.clone();
                        grid.spawn((
                            Node::default(),
                            Interaction::default(),
                            bevy_ui_widgets::Button,
                            ClassList::new("pane-button-grid-item"),
                            GridButton {
                                index: i,
                                label: label.clone(),
                            },
                            bevy_ui_widgets::observe(
                                move |_: On<bevy_ui_widgets::Activate>, mut commands: Commands| {
                                    commands.trigger(PaneButtonPressed {
                                        pane: meta_for_btn.pane_title.clone(),
                                        label: btn_label.clone(),
                                    });
                                },
                            ),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(label),
                                pane_font(10.0),
                                ClassList::new("pane-button-grid-item-text"),
                            ));
                        });
                    }
                });
        });

    row_entity
}

// ══════════════════════════════════════════════════════════════════════
// Builder extension
// ══════════════════════════════════════════════════════════════════════

fn button_grid_config(buttons: &[&str], columns: usize) -> ControlConfig {
    ControlConfig::new()
        .string_list("labels", buttons.iter().map(|s| s.to_string()).collect())
        .int("columns", columns as i64)
}

pub trait ButtonGridPaneExt {
    fn button_grid(self, label: &str, buttons: &[&str]) -> Self;
    fn button_grid_columns(self, label: &str, buttons: &[&str], columns: usize) -> Self;
}

impl ButtonGridPaneExt for saddle_pane::prelude::PaneBuilder {
    fn button_grid(self, label: &str, buttons: &[&str]) -> Self {
        self.custom("button_grid", label, button_grid_config(buttons, 3))
    }

    fn button_grid_columns(self, label: &str, buttons: &[&str], columns: usize) -> Self {
        self.custom("button_grid", label, button_grid_config(buttons, columns))
    }
}

impl ButtonGridPaneExt for saddle_pane::builder::FolderBuilder {
    fn button_grid(self, label: &str, buttons: &[&str]) -> Self {
        self.custom("button_grid", label, button_grid_config(buttons, 3))
    }

    fn button_grid_columns(self, label: &str, buttons: &[&str], columns: usize) -> Self {
        self.custom("button_grid", label, button_grid_config(buttons, columns))
    }
}
