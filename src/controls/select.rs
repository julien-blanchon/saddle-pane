use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui_widgets::{Activate, MenuAction, MenuButton, MenuEvent, MenuItem, MenuPopup};

use super::{PaneControlMeta, value_font};

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/select.css";
const POPUP_STYLE_PATH: &str = "embedded://saddle_pane/style/select_popup.css";

/// Component storing select/dropdown state.
#[derive(Component, Clone, Debug)]
pub struct SelectControl {
    pub value: usize,
    pub options: Vec<String>,
}

/// Whether the dropdown popup is open.
#[derive(Component, Clone, Debug, Default)]
pub struct SelectOpen(pub bool);

/// Marker for select control rows.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct SelectControlMarker;

/// Wrapper node that contains trigger + popup (for correct positioning).
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct SelectWrapper;

/// Marker for the trigger button inside the select.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct SelectTrigger;

/// Marker for the selected label text.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct SelectLabel;

/// Stored on each menu item to track which option index it represents.
#[derive(Component, Clone, Debug)]
pub(crate) struct SelectItemIndex(pub usize);

/// Stored on popup to track owning select entity.
#[derive(Component, Clone, Debug)]
pub(crate) struct SelectPopupOwner(pub Entity);

/// Spawn select/dropdown UI as children of `parent`.
pub(crate) fn spawn_select_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &SelectControl,
    asset_server: &AssetServer,
) -> Entity {
    let label_text = control
        .options
        .get(control.value)
        .cloned()
        .unwrap_or_default();

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
            SelectControlMarker,
            SelectOpen::default(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Label
            super::spawn_label_with_icon(row, &meta.label, meta.icon.as_deref(), meta.icon_handle.clone());

            // Wrapper: contains trigger + popup, position: relative for popup alignment
            row.spawn((
                Node::default(),
                SelectWrapper,
                ClassList::new("pane-select-wrapper"),
            ))
            .with_children(|wrapper| {
                // Trigger — MenuButton for proper menu event handling
                wrapper
                    .spawn((
                        Node::default(),
                        Interaction::default(),
                        MenuButton,
                        SelectTrigger,
                        ClassList::new("pane-select-trigger"),
                        AutoDirectionalNavigation::default(),
                        TabIndex(0),
                    ))
                    .with_children(|trigger| {
                        trigger.spawn((
                            Text::new(label_text),
                            value_font(),
                            ClassList::new("pane-select-label"),
                            SelectLabel,
                        ));
                        trigger.spawn((Node::default(), ClassList::new("pane-select-arrow")));
                    });
            });
        });

    row_entity
}

/// Spawn dropdown popup as a root-level entity (avoids overflow:clip from parent pane).
/// Positioned absolutely and tracked by `SelectPopupOwner`.
fn spawn_select_popup(
    commands: &mut Commands,
    select_entity: Entity,
    control: &SelectControl,
    asset_server: &AssetServer,
    _wrapper_entity: Entity,
) {
    commands
        .spawn((
            Node::default(),
            MenuPopup::default(),
            SelectPopupOwner(select_entity),
            ClassList::new("pane-select-popup"),
            NodeStyleSheet::new(asset_server.load(POPUP_STYLE_PATH)),
            GlobalZIndex(1000),
        ))
            .with_children(|popup| {
                for (i, option) in control.options.iter().enumerate() {
                    popup
                        .spawn((
                            Node::default(),
                            Interaction::default(),
                            MenuItem,
                            SelectItemIndex(i),
                            ClassList::new("pane-select-item"),
                            AutoDirectionalNavigation::default(),
                            TabIndex(0),
                        ))
                        .with_children(|item| {
                            item.spawn((Text::new(option.clone()), value_font()));
                        });
                }
            });
}

/// System: sync SelectOpen → spawn/despawn popup.
pub(crate) fn sync_select_open(
    q_selects: Query<
        (Entity, &SelectOpen, &SelectControl),
        (With<SelectControlMarker>, Changed<SelectOpen>),
    >,
    q_popup: Query<(Entity, &SelectPopupOwner)>,
    q_children: Query<&Children>,
    q_wrapper: Query<Entity, With<SelectWrapper>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (select_entity, open, control) in &q_selects {
        let popup_entity = q_popup
            .iter()
            .find(|(_, owner)| owner.0 == select_entity)
            .map(|(e, _)| e);

        match (open.0, popup_entity) {
            (true, None) => {
                // Find the wrapper child of this row
                if let Ok(children) = q_children.get(select_entity) {
                    for child in children.iter() {
                        if q_wrapper.contains(child) {
                            spawn_select_popup(
                                &mut commands,
                                select_entity,
                                control,
                                &asset_server,
                                child,
                            );
                            break;
                        }
                    }
                }
            }
            (false, Some(popup)) => {
                commands.entity(popup).despawn();
            }
            _ => {}
        }
    }
}

/// Observer: handle MenuEvent from trigger (toggle) and popup (close).
pub(crate) fn on_select_menu_event(
    ev: On<MenuEvent>,
    q_trigger: Query<&ChildOf, With<SelectTrigger>>,
    q_popup: Query<(Entity, &SelectPopupOwner)>,
    q_parent: Query<&ChildOf>,
    mut q_open: Query<&mut SelectOpen, With<SelectControlMarker>>,
) {
    match ev.event().action {
        MenuAction::Toggle => {
            // Trigger → wrapper → row (select entity)
            let Ok(child_of) = q_trigger.get(ev.event().source) else {
                return;
            };
            let wrapper_entity = child_of.parent();
            let Ok(wrapper_parent) = q_parent.get(wrapper_entity) else {
                return;
            };
            let select_entity = wrapper_parent.parent();
            if let Ok(mut open) = q_open.get_mut(select_entity) {
                open.0 = !open.0;
            }
        }
        MenuAction::CloseAll => {
            let mut current = ev.event().source;
            loop {
                if let Ok((_, owner)) = q_popup.get(current) {
                    if let Ok(mut open) = q_open.get_mut(owner.0) {
                        open.0 = false;
                    }
                    return;
                }
                if let Ok(child_of) = q_parent.get(current) {
                    current = child_of.parent();
                } else {
                    break;
                }
            }
        }
        _ => {}
    }
}

/// Observer: handle Activate from menu items to update the selected value.
pub(crate) fn on_select_item_activate(
    ev: On<Activate>,
    q_item: Query<&SelectItemIndex>,
    q_parent: Query<&ChildOf>,
    q_popup: Query<&SelectPopupOwner>,
    mut q_select: Query<&mut SelectControl, With<SelectControlMarker>>,
    mut q_open: Query<&mut SelectOpen, With<SelectControlMarker>>,
) {
    let Ok(index) = q_item.get(ev.entity) else {
        return;
    };

    // Walk up to find the SelectPopupOwner
    let mut current = ev.entity;
    let owner = loop {
        if let Ok(popup_owner) = q_popup.get(current) {
            break Some(popup_owner.0);
        }
        if let Ok(child_of) = q_parent.get(current) {
            current = child_of.parent();
        } else {
            break None;
        }
    };

    let Some(select_entity) = owner else { return };

    if let Ok(mut control) = q_select.get_mut(select_entity) {
        control.value = index.0;
        if let Ok(mut open) = q_open.get_mut(select_entity) {
            open.0 = false;
        }
    }
}

/// System: position root-level select popup below its owner's wrapper.
pub(crate) fn position_select_popup(
    q_popups: Query<(Entity, &SelectPopupOwner)>,
    q_children: Query<&Children>,
    q_wrapper: Query<(&UiGlobalTransform, &ComputedNode), With<SelectWrapper>>,
    mut q_node: Query<&mut Node>,
) {
    for (popup_entity, owner) in &q_popups {
        // Find the SelectWrapper child of the owner row
        let Ok(children) = q_children.get(owner.0) else {
            continue;
        };
        let Some((global_transform, computed)) = children
            .iter()
            .find_map(|child| q_wrapper.get(child).ok())
        else {
            continue;
        };
        let Ok(mut node) = q_node.get_mut(popup_entity) else {
            continue;
        };

        let center = global_transform.affine().translation;
        let size = computed.size();
        let inv_scale = computed.inverse_scale_factor;

        // Position below the wrapper, aligned to its left edge
        let left = (center.x - size.x / 2.0) * inv_scale;
        let top = (center.y + size.y / 2.0) * inv_scale;
        let width = size.x * inv_scale;

        node.position_type = PositionType::Absolute;
        node.left = Val::Px(left);
        node.top = Val::Px(top);
        node.width = Val::Px(width);
    }
}

/// System: keep label text in sync with SelectControl.
pub(crate) fn update_select_label(
    q_selects: Query<(Entity, &SelectControl), (With<SelectControlMarker>, Changed<SelectControl>)>,
    q_children: Query<&Children>,
    mut q_label: Query<&mut Text, With<SelectLabel>>,
) {
    for (select_entity, control) in &q_selects {
        if let Some(text) = control.options.get(control.value) {
            for descendant in q_children.iter_descendants(select_entity) {
                if let Ok(mut label) = q_label.get_mut(descendant) {
                    label.0 = text.clone();
                    break;
                }
            }
        }
    }
}
