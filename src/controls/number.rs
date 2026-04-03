use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_ui_widgets::{Activate, Button};

/// Initial delay before repeat fires (seconds).
const REPEAT_INITIAL_DELAY: f32 = 0.4;
/// Interval between repeat fires (seconds).
const REPEAT_INTERVAL: f32 = 0.08;

use super::editing::{
    PaneEditOwner, PaneEditState, PaneEditableDisplay, PaneNumberEdit, format_value,
};
use super::{PaneControlMeta, value_font};

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/number.css";

/// Component storing number input state.
#[derive(Component, Clone, Debug)]
pub struct NumberControl {
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: f64,
    pub show_step_buttons: bool,
}

/// Marker for the number row entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct NumberControlMarker;

/// Step button component (- or +) with repeat-on-hold state.
#[derive(Component, Clone, Debug)]
pub(crate) struct NumberStepButton {
    pub direction: f64, // -1.0 or +1.0
    held: bool,
    timer: f32,
    initial_fired: bool,
}

impl NumberStepButton {
    fn new(direction: f64) -> Self {
        Self {
            direction,
            held: false,
            timer: 0.0,
            initial_fired: false,
        }
    }
}

/// Spawn number input UI as children of `parent`.
pub(crate) fn spawn_number_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &NumberControl,
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
            NumberControlMarker,
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

            // [-] step button
            if control.show_step_buttons {
                row.spawn((
                    Node::default(),
                    Interaction::default(),
                    Button,
                    NumberStepButton::new(-1.0),
                    ClassList::new("pane-number-step"),
                    AutoDirectionalNavigation::default(),
                    TabIndex(0),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("-"),
                        value_font(),
                        ClassList::new("pane-number-step-text"),
                    ));
                });
            }

            // Editable number value
            let mut editor_cmd = row.spawn((
                Node::default(),
                Interaction::default(),
                AutoDirectionalNavigation::default(),
                TabIndex(0),
                ClassList::new("pane-number"),
                PaneEditState::new(format_value(control.value)),
                PaneNumberEdit {
                    step: control.step,
                    min: control.min,
                    max: control.max,
                },
            ));

            editor_cmd.with_children(|input| {
                input.spawn((
                    Text::new(format_value(control.value)),
                    value_font(),
                    ClassList::new("pane-number-value"),
                    PaneEditableDisplay,
                ));
            });

            editor_entity = editor_cmd.id();

            // [+] step button
            if control.show_step_buttons {
                row.spawn((
                    Node::default(),
                    Interaction::default(),
                    Button,
                    NumberStepButton::new(1.0),
                    ClassList::new("pane-number-step"),
                    AutoDirectionalNavigation::default(),
                    TabIndex(0),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("+"),
                        value_font(),
                        ClassList::new("pane-number-step-text"),
                    ));
                });
            }
        });

    // Link editor to row
    parent
        .commands()
        .entity(editor_entity)
        .insert(PaneEditOwner(row_entity));

    row_entity
}

/// Observer: handle step button clicks (increment/decrement number value).
pub(crate) fn on_number_step(
    ev: On<Activate>,
    q_step: Query<(&NumberStepButton, &ChildOf)>,
    mut q_number: Query<&mut NumberControl, With<NumberControlMarker>>,
) {
    let Ok((step_btn, child_of)) = q_step.get(ev.entity) else {
        return;
    };
    let row_entity = child_of.parent();
    let Ok(mut ctrl) = q_number.get_mut(row_entity) else {
        return;
    };
    apply_step(&mut ctrl, step_btn.direction);
}

/// System (Update): repeat-on-hold for step buttons.
pub(crate) fn update_number_step_repeat(
    time: Res<Time>,
    mut q_step: Query<(&mut NumberStepButton, &Interaction, &ChildOf)>,
    mut q_number: Query<&mut NumberControl, With<NumberControlMarker>>,
) {
    let dt = time.delta_secs();
    for (mut step_btn, interaction, child_of) in &mut q_step {
        match *interaction {
            Interaction::Pressed => {
                if !step_btn.held {
                    // Just pressed — start hold tracking
                    step_btn.held = true;
                    step_btn.timer = 0.0;
                    step_btn.initial_fired = false;
                } else {
                    step_btn.timer += dt;
                    if !step_btn.initial_fired {
                        // Wait for initial delay
                        if step_btn.timer >= REPEAT_INITIAL_DELAY {
                            step_btn.initial_fired = true;
                            step_btn.timer = 0.0;
                            if let Ok(mut ctrl) = q_number.get_mut(child_of.parent()) {
                                apply_step(&mut ctrl, step_btn.direction);
                            }
                        }
                    } else if step_btn.timer >= REPEAT_INTERVAL {
                        // Repeat at interval
                        step_btn.timer -= REPEAT_INTERVAL;
                        if let Ok(mut ctrl) = q_number.get_mut(child_of.parent()) {
                            apply_step(&mut ctrl, step_btn.direction);
                        }
                    }
                }
            }
            _ => {
                step_btn.held = false;
                step_btn.timer = 0.0;
                step_btn.initial_fired = false;
            }
        }
    }
}

fn apply_step(ctrl: &mut NumberControl, direction: f64) {
    let mut val = ctrl.value + direction * ctrl.step;
    if let Some(min) = ctrl.min {
        val = val.max(min);
    }
    if let Some(max) = ctrl.max {
        val = val.min(max);
    }
    ctrl.value = val;
}
