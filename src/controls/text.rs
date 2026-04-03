use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;

use super::editing::{PaneEditOwner, PaneEditState, PaneEditableDisplay, PaneTextEdit};
use super::{PaneControlMeta, value_font};

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/text.css";

/// Component storing text input state.
#[derive(Component, Clone, Debug)]
pub struct TextControl {
    pub value: String,
}

/// Marker for the text control row.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct TextControlMarker;

/// Spawn text input UI as children of `parent`.
pub(crate) fn spawn_text_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &TextControl,
    asset_server: &AssetServer,
) -> Entity {
    let mut row_entity = Entity::PLACEHOLDER;
    let mut editor_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
            TextControlMarker,
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Label
            super::spawn_label_with_icon(row, &meta.label, meta.icon.as_deref(), meta.icon_handle.clone());

            // Editable text value
            let mut editor_cmd = row.spawn((
                Node::default(),
                Interaction::default(),
                AutoDirectionalNavigation::default(),
                TabIndex(0),
                ClassList::new("pane-text-input"),
                PaneEditState::new(&control.value),
                PaneTextEdit,
            ));

            editor_cmd.with_children(|input| {
                input.spawn((
                    Text::new(&control.value),
                    value_font(),
                    ClassList::new("pane-text-value"),
                    PaneEditableDisplay,
                ));
            });

            editor_entity = editor_cmd.id();
        });

    // Link editor to row
    parent
        .commands()
        .entity(editor_entity)
        .insert(PaneEditOwner(row_entity));

    row_entity
}
