use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui_widgets::Checkbox;

use bevy::ui::Checked;

use super::PaneControlMeta;

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/toggle.css";

/// Component storing toggle control state.
#[derive(Component, Clone, Debug)]
pub struct ToggleControl {
    pub value: bool,
}

/// Link from a control row to its inner checkbox widget entity.
#[derive(Component, Clone, Debug)]
pub(crate) struct ToggleWidgetLink(pub Entity);

/// Spawn toggle UI as children of `parent`.
pub(crate) fn spawn_toggle_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &ToggleControl,
    asset_server: &AssetServer,
) -> Entity {
    let initial_value = control.value;
    let mut row_entity = Entity::PLACEHOLDER;
    let mut toggle_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Label
            super::spawn_label_with_icon(
                row,
                &meta.label,
                meta.icon.as_deref(),
                meta.icon_handle.clone(),
            );

            // Checkbox (using Checkbox headless widget)
            let mut toggle_cmd = row.spawn((
                Node::default(),
                Interaction::default(),
                Checkbox,
                ClassList::new("pane-toggle"),
                AutoDirectionalNavigation::default(),
                TabIndex(0),
            ));

            if initial_value {
                toggle_cmd.insert(Checked);
            }

            toggle_cmd.with_children(|toggle| {
                toggle.spawn((Node::default(), ClassList::new("pane-toggle-knob")));
            });

            toggle_entity = toggle_cmd.id();
        });

    parent
        .commands()
        .entity(row_entity)
        .insert(ToggleWidgetLink(toggle_entity));

    row_entity
}

/// System: sync Checked component → ToggleControl.
pub(crate) fn sync_toggle_to_control(
    mut q_links: Query<(&ToggleWidgetLink, &mut ToggleControl)>,
    q_checked: Query<Has<Checked>, Changed<Checked>>,
) {
    for (link, mut control) in q_links.iter_mut() {
        if let Ok(is_checked) = q_checked.get(link.0) {
            control.value = is_checked;
        }
    }
}
