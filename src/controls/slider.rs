use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue, TrackClick};

use super::editing::{
    PaneEditOwner, PaneEditState, PaneEditableDisplay, PaneSliderEdit, format_value,
};
use super::{PaneControlMeta, css_percent, value_font};

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/slider.css";

/// Component storing slider control state.
#[derive(Component, Clone, Debug)]
pub struct SliderControl {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// Link from a control row to its inner slider widget entity.
#[derive(Component, Clone, Debug)]
pub(crate) struct SliderWidgetLink(pub Entity);

/// Marker on the slider value text.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct SliderValueText;

/// Spawn slider UI as children of `parent`.
pub(crate) fn spawn_slider_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &SliderControl,
    asset_server: &AssetServer,
) -> Entity {
    let range = SliderRange::new(control.min as f32, control.max as f32);
    let fill_pct = range.thumb_position(control.value as f32) * 100.0;

    let mut row_entity = Entity::PLACEHOLDER;
    let mut slider_entity = Entity::PLACEHOLDER;
    let mut editor_entity = Entity::PLACEHOLDER;

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

            // Label (width set in Rust for reliable alignment)
            super::spawn_label_with_icon(
                row,
                &meta.label,
                meta.icon.as_deref(),
                meta.icon_handle.clone(),
            );

            // Slider widget
            let mut slider_cmd = row.spawn((
                Node::default(),
                Interaction::default(),
                Slider {
                    track_click: TrackClick::Drag,
                },
                SliderValue(control.value as f32),
                SliderStep(control.step as f32),
                range,
                ClassList::new("pane-slider"),
                AutoDirectionalNavigation::default(),
                TabIndex(0),
                InlineStyle::from_iter([
                    ("--slider-fill-width", css_percent(fill_pct)),
                    ("--slider-thumb-left", css_percent(fill_pct)),
                ]),
            ));

            slider_cmd.with_children(|slider| {
                // Track (relative positioned, thin line)
                slider
                    .spawn((
                        Node::default(),
                        Interaction::default(),
                        ClassList::new("pane-slider-track"),
                    ))
                    .with_children(|track| {
                        // Fill bar
                        track.spawn((Node::default(), ClassList::new("pane-slider-fill")));
                        // Thumb
                        track.spawn((
                            Node::default(),
                            Interaction::default(),
                            SliderThumb,
                            ClassList::new("pane-slider-thumb"),
                        ));
                    });
            });

            slider_entity = slider_cmd.id();

            // Value display — editable text field
            let mut value_cmd = row.spawn((
                Node::default(),
                Interaction::default(),
                AutoDirectionalNavigation::default(),
                TabIndex(0),
                ClassList::new("pane-slider-value"),
                PaneEditState::new(format_value(control.value)),
                PaneSliderEdit {
                    step: control.step,
                    min: control.min,
                    max: control.max,
                },
            ));

            value_cmd.with_children(|val| {
                val.spawn((
                    Text::new(format_value(control.value)),
                    value_font(),
                    ClassList::new("pane-slider-value-text"),
                    SliderValueText,
                    PaneEditableDisplay,
                ));
            });

            editor_entity = value_cmd.id();
        });

    // Store link from row to slider widget
    parent
        .commands()
        .entity(row_entity)
        .insert(SliderWidgetLink(slider_entity));
    // Link editor to row
    parent
        .commands()
        .entity(editor_entity)
        .insert(PaneEditOwner(row_entity));

    row_entity
}

/// System: update slider InlineStyle when SliderValue changes (from user drag).
pub(crate) fn update_slider_fill(
    mut q: Query<
        (&SliderValue, &SliderRange, &mut InlineStyle),
        (
            With<Slider>,
            Or<(Changed<SliderValue>, Changed<SliderRange>)>,
        ),
    >,
) {
    for (value, range, mut style) in &mut q {
        let pct = css_percent(range.thumb_position(value.0) * 100.0);
        style.set("--slider-fill-width", pct.clone());
        style.set("--slider-thumb-left", pct);
    }
}

/// System: update slider value text when SliderValue changes (from drag).
pub(crate) fn update_slider_value_text(
    q_sliders: Query<(Entity, &SliderValue), (With<Slider>, Changed<SliderValue>)>,
    q_children: Query<&Children>,
    mut q_text: Query<&mut Text, With<SliderValueText>>,
    q_parent: Query<&ChildOf>,
) {
    for (slider_entity, value) in &q_sliders {
        // Walk up to the row entity (parent of slider), then find SliderValueText descendant
        if let Ok(child_of) = q_parent.get(slider_entity) {
            let row = child_of.parent();
            for descendant in q_children.iter_descendants(row) {
                if let Ok(mut text) = q_text.get_mut(descendant) {
                    text.0 = format_value(value.0 as f64);
                    break;
                }
            }
        }
    }
}

/// System: sync SliderValue (widget) → SliderControl (our component).
pub(crate) fn sync_slider_to_control(
    mut q_links: Query<(&SliderWidgetLink, &mut SliderControl)>,
    q_sliders: Query<&SliderValue, Changed<SliderValue>>,
) {
    for (link, mut control) in q_links.iter_mut() {
        if let Ok(slider_val) = q_sliders.get(link.0) {
            control.value = slider_val.0 as f64;
        }
    }
}
