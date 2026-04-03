use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;

use super::color_picker::ColorPickerOpen;
use super::editing::{
    PaneEditOwner, PaneEditState, PaneEditableDisplay, PaneHexEdit, color_to_hex,
};
use super::{PaneControlMeta, css_color, value_font};

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/color.css";

/// Component storing color control state.
#[derive(Component, Clone, Debug)]
pub struct ColorControl {
    pub value: Color,
}

/// Marker for the color swatch entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorSwatch;

/// Marker for the color control row.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorControlMarker;

/// Wrapper node for swatch + hex + picker popup.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorAreaWrapper;

/// Spawn color control UI as children of `parent`.
pub(crate) fn spawn_color_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &ColorControl,
    asset_server: &AssetServer,
) -> Entity {
    let color_str = css_color(control.value);
    let hex_str = color_to_hex(control.value);

    let mut row_entity = Entity::PLACEHOLDER;
    let mut editor_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
            ColorControlMarker,
            ColorPickerOpen::default(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Label
            super::spawn_label_with_icon(row, &meta.label, meta.icon.as_deref(), meta.icon_handle.clone());

            // Color value area (swatch + hex + picker popup wrapper)
            row.spawn((
                Node {
                    display: Display::Flex,
                    position_type: PositionType::Relative,
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                },
                ClassList::new("pane-color-area"),
                ColorAreaWrapper,
            ))
            .with_children(|area| {
                // Color swatch — clickable, focuses hex editor
                area.spawn((
                    Node::default(),
                    Interaction::default(),
                    ClassList::new("pane-color-swatch"),
                    InlineStyle::from_iter([("--swatch-color", color_str)]),
                    ColorSwatch,
                ));

                // Editable hex value
                let mut hex_cmd = area.spawn((
                    Node::default(),
                    Interaction::default(),
                    AutoDirectionalNavigation::default(),
                    TabIndex(0),
                    ClassList::new("pane-color-hex"),
                    PaneEditState::new(&hex_str),
                    PaneHexEdit,
                ));

                hex_cmd.with_children(|hex| {
                    hex.spawn((
                        Text::new(hex_str),
                        value_font(),
                        ClassList::new("pane-color-hex-text"),
                        PaneEditableDisplay,
                    ));
                });

                editor_entity = hex_cmd.id();
            });
        });

    // Link editor to row
    parent
        .commands()
        .entity(editor_entity)
        .insert(PaneEditOwner(row_entity));

    row_entity
}

/// System: update color swatch when ColorControl changes (hex text handled by editing system).
pub(crate) fn update_color_display(
    q_colors: Query<(Entity, &ColorControl), (With<ColorControlMarker>, Changed<ColorControl>)>,
    q_children: Query<&Children>,
    mut q_swatch: Query<&mut InlineStyle, With<ColorSwatch>>,
) {
    for (entity, control) in &q_colors {
        let color_str = css_color(control.value);

        for descendant in q_children.iter_descendants(entity) {
            if let Ok(mut style) = q_swatch.get_mut(descendant) {
                style.set("--swatch-color", color_str.clone());
            }
        }
    }
}
