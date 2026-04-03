//! Search/filter system for pane controls.

use bevy::prelude::*;
use bevy_flair::prelude::ClassList;

use crate::controls::PaneControlMeta;
use crate::layout::{PaneFolder, PaneFolderBody, PaneRoot, PaneSearchFilter, PaneSearchText};

/// System: apply search filter to hide non-matching control rows.
/// Runs when `PaneSearchFilter` changes on any PaneRoot.
pub(crate) fn apply_search_filter(
    q_panes: Query<(Entity, &PaneSearchFilter), (With<PaneRoot>, Changed<PaneSearchFilter>)>,
    q_children: Query<&Children>,
    q_meta: Query<&PaneControlMeta>,
    mut q_classes: Query<&mut ClassList>,
    q_folder: Query<&PaneFolder>,
    q_folder_body: Query<(), With<PaneFolderBody>>,
) {
    for (pane_entity, filter) in &q_panes {
        let query = filter.0.to_lowercase();

        // Iterate all descendants, show/hide control rows based on label match
        let descendants: Vec<Entity> = q_children.iter_descendants(pane_entity).collect();

        for &entity in &descendants {
            if let Ok(meta) = q_meta.get(entity) {
                let visible = query.is_empty() || meta.label.to_lowercase().contains(&query);
                if let Ok(mut classes) = q_classes.get_mut(entity) {
                    if visible {
                        classes.remove("is-hidden");
                    } else {
                        classes.add("is-hidden");
                    }
                }
            }
        }

        // Hide folders when ALL their children are hidden
        for &entity in &descendants {
            if q_folder.contains(entity) {
                let mut any_child_visible = false;
                if let Ok(children) = q_children.get(entity) {
                    for child in children.iter() {
                        // Check folder body's children
                        if q_folder_body.contains(child) {
                            if let Ok(body_children) = q_children.get(child) {
                                for ctrl in body_children.iter() {
                                    if let Ok(classes) = q_classes.get(ctrl) {
                                        if !classes.contains("is-hidden") {
                                            any_child_visible = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if any_child_visible {
                            break;
                        }
                    }
                }
                if let Ok(mut classes) = q_classes.get_mut(entity) {
                    if query.is_empty() || any_child_visible {
                        classes.remove("is-hidden");
                    } else {
                        classes.add("is-hidden");
                    }
                }
            }
        }
    }
}

/// System: sync search input text → PaneSearchFilter on parent PaneRoot.
pub(crate) fn sync_search_input(
    q_search: Query<(&Text, &ChildOf), (With<PaneSearchText>, Changed<Text>)>,
    q_parent: Query<&ChildOf>,
    mut q_filter: Query<&mut PaneSearchFilter, With<PaneRoot>>,
) {
    for (text, search_bar_of) in &q_search {
        // PaneSearchText → search-bar → PaneRoot
        let search_bar = search_bar_of.parent();
        if let Ok(bar_parent) = q_parent.get(search_bar) {
            if let Ok(mut filter) = q_filter.get_mut(bar_parent.parent()) {
                if filter.0 != text.0 {
                    filter.0.clone_from(&text.0);
                }
            }
        }
    }
}
