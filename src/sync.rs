use bevy::prelude::*;
use bevy::ui::Checked;
use bevy_ui_widgets::SliderValue;

use crate::controls::PaneControlMeta;
use crate::controls::PaneValue;
use crate::controls::color::{ColorControl, ColorControlMarker};
use crate::controls::number::{NumberControl, NumberControlMarker};
use crate::controls::select::{SelectControl, SelectControlMarker};
use crate::controls::slider::{SliderControl, SliderWidgetLink};
use crate::controls::text::{TextControl, TextControlMarker};
use crate::controls::toggle::{ToggleControl, ToggleWidgetLink};
use crate::events::PaneChanged;
use crate::store::PaneStore;

/// Helper: sync a changed control value to the store and emit PaneChanged.
fn sync_one(
    store: &mut PaneStore,
    commands: &mut Commands,
    meta: &PaneControlMeta,
    value: PaneValue,
) {
    if store.get_raw(&meta.pane_title, &meta.label) != Some(&value) {
        store.set_from_ui(&meta.pane_title, &meta.label, value.clone());
        commands.trigger(PaneChanged {
            pane: meta.pane_title.clone(),
            field: meta.label.clone(),
            value,
        });
    }
}

/// Sync all control component values → PaneStore + emit PaneChanged events.
pub(crate) fn sync_controls_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q_sliders: Query<(&PaneControlMeta, &SliderControl), Changed<SliderControl>>,
    q_toggles: Query<(&PaneControlMeta, &ToggleControl), Changed<ToggleControl>>,
    q_numbers: Query<
        (&PaneControlMeta, &NumberControl),
        (With<NumberControlMarker>, Changed<NumberControl>),
    >,
    q_texts: Query<
        (&PaneControlMeta, &TextControl),
        (With<TextControlMarker>, Changed<TextControl>),
    >,
    q_selects: Query<
        (&PaneControlMeta, &SelectControl),
        (With<SelectControlMarker>, Changed<SelectControl>),
    >,
    q_colors: Query<
        (&PaneControlMeta, &ColorControl),
        (With<ColorControlMarker>, Changed<ColorControl>),
    >,
) {
    for (meta, ctrl) in &q_sliders {
        sync_one(
            &mut store,
            &mut commands,
            meta,
            PaneValue::Float(ctrl.value),
        );
    }
    for (meta, ctrl) in &q_toggles {
        sync_one(&mut store, &mut commands, meta, PaneValue::Bool(ctrl.value));
    }
    for (meta, ctrl) in &q_numbers {
        sync_one(
            &mut store,
            &mut commands,
            meta,
            PaneValue::Float(ctrl.value),
        );
    }
    for (meta, ctrl) in &q_texts {
        sync_one(
            &mut store,
            &mut commands,
            meta,
            PaneValue::String(ctrl.value.clone()),
        );
    }
    for (meta, ctrl) in &q_selects {
        sync_one(
            &mut store,
            &mut commands,
            meta,
            PaneValue::Int(ctrl.value as i64),
        );
    }
    for (meta, ctrl) in &q_colors {
        sync_one(
            &mut store,
            &mut commands,
            meta,
            PaneValue::Color(ctrl.value),
        );
    }
}

/// Helper: build the dirty-key for a control.
fn dirty_key(meta: &PaneControlMeta) -> (String, String) {
    (meta.pane_title.clone(), meta.label.clone())
}

/// Sync dirty PaneStore values → control components (reverse direction).
/// This runs after `sync_controls_to_store` so external `store.set()` calls
/// propagate to the UI.
pub(crate) fn sync_store_to_controls(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    mut q_sliders: Query<(&PaneControlMeta, &mut SliderControl, &SliderWidgetLink)>,
    mut q_toggles: Query<
        (&PaneControlMeta, &mut ToggleControl, &ToggleWidgetLink),
        Without<SliderControl>,
    >,
    mut q_numbers: Query<
        (&PaneControlMeta, &mut NumberControl),
        (Without<SliderControl>, Without<ToggleControl>),
    >,
    mut q_texts: Query<
        (&PaneControlMeta, &mut TextControl),
        (
            Without<SliderControl>,
            Without<ToggleControl>,
            Without<NumberControl>,
        ),
    >,
    mut q_selects: Query<
        (&PaneControlMeta, &mut SelectControl),
        (
            Without<SliderControl>,
            Without<ToggleControl>,
            Without<NumberControl>,
            Without<TextControl>,
        ),
    >,
    mut q_colors: Query<
        (&PaneControlMeta, &mut ColorControl),
        (
            Without<SliderControl>,
            Without<ToggleControl>,
            Without<NumberControl>,
            Without<TextControl>,
            Without<SelectControl>,
        ),
    >,
) {
    if !store.has_dirty() {
        return;
    }

    let dirty = store.drain_dirty();

    for (meta, mut ctrl, link) in &mut q_sliders {
        if dirty.contains(&dirty_key(meta)) {
            if let Some(PaneValue::Float(v)) = store.get_raw(&meta.pane_title, &meta.label) {
                if ctrl.value != *v {
                    ctrl.value = *v;
                    commands.entity(link.0).insert(SliderValue(*v as f32));
                }
            }
        }
    }

    for (meta, mut ctrl, link) in &mut q_toggles {
        if dirty.contains(&dirty_key(meta)) {
            if let Some(PaneValue::Bool(v)) = store.get_raw(&meta.pane_title, &meta.label) {
                if ctrl.value != *v {
                    ctrl.value = *v;
                    if *v {
                        commands.entity(link.0).insert(Checked);
                    } else {
                        commands.entity(link.0).remove::<Checked>();
                    }
                }
            }
        }
    }

    for (meta, mut ctrl) in &mut q_numbers {
        if dirty.contains(&dirty_key(meta)) {
            if let Some(PaneValue::Float(v)) = store.get_raw(&meta.pane_title, &meta.label) {
                if ctrl.value != *v {
                    ctrl.value = *v;
                }
            }
        }
    }

    for (meta, mut ctrl) in &mut q_texts {
        if dirty.contains(&dirty_key(meta)) {
            if let Some(PaneValue::String(v)) = store.get_raw(&meta.pane_title, &meta.label) {
                if ctrl.value != *v {
                    ctrl.value.clone_from(v);
                }
            }
        }
    }

    for (meta, mut ctrl) in &mut q_selects {
        if dirty.contains(&dirty_key(meta)) {
            if let Some(PaneValue::Int(v)) = store.get_raw(&meta.pane_title, &meta.label) {
                let idx = *v as usize;
                if ctrl.value != idx {
                    ctrl.value = idx;
                }
            }
        }
    }

    for (meta, mut ctrl) in &mut q_colors {
        if dirty.contains(&dirty_key(meta)) {
            if let Some(PaneValue::Color(v)) = store.get_raw(&meta.pane_title, &meta.label) {
                if ctrl.value != *v {
                    ctrl.value = *v;
                }
            }
        }
    }
}
