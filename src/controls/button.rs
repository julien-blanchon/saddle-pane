use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui_widgets::{Activate, Button, observe};

use super::{PaneControlMeta, value_font};
use crate::events::PaneButtonPressed;

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/button.css";

/// Marker component for button controls.
#[derive(Component, Clone, Debug, Default)]
pub struct ButtonControl;

/// Spawn button UI as children of `parent`.
pub(crate) fn spawn_button_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    asset_server: &AssetServer,
) -> Entity {
    let pane_title = meta.pane_title.clone();
    let label = meta.label.clone();

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row pane-row-button"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            ButtonControl,
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            row.spawn((
                Node::default(),
                Interaction::default(),
                Button,
                ClassList::new("pane-button"),
                AutoDirectionalNavigation::default(),
                TabIndex(0),
                observe(move |_ev: On<Activate>, mut commands: Commands| {
                    commands.trigger(PaneButtonPressed {
                        pane: pane_title.clone(),
                        label: label.clone(),
                    });
                }),
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(&meta.label),
                    value_font(),
                    ClassList::new("pane-button-text"),
                ));
            });
        });

    row_entity
}
