//! Inline text editing for pane controls (number, text, hex color).
//!
//! Modeled after the `text_input` bevy-ui example but simplified:
//! no clipboard, no selection, just caret editing with commit on Enter/blur.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::prelude::*;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_input_focus::{FocusedInput, InputFocus};

use super::PaneControlMeta;
use super::PaneValue;
use super::color::{ColorControl, ColorSwatch};
use super::css_color;
use super::number::NumberControl;
use super::slider::{SliderControl, SliderWidgetLink};
use super::text::TextControl;
use crate::events::{PaneEditEnd, PaneEditStart};
use bevy_ui_widgets::SliderValue;

// ── Components ──────────────────────────────────────────────

/// Tracks inline text editing state for a pane control value.
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneEditState {
    pub buffer: String,
    pub original: String,
    pub caret: usize,
}

impl PaneEditState {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let caret = value.len();
        Self {
            buffer: value.clone(),
            original: value,
            caret,
        }
    }

    fn move_left(&mut self) {
        self.caret = previous_boundary(&self.buffer, self.caret);
    }

    fn move_right(&mut self) {
        self.caret = next_boundary(&self.buffer, self.caret);
    }

    fn insert_text(&mut self, text: &str) {
        self.buffer.insert_str(self.caret, text);
        self.caret += text.len();
    }

    fn delete_backward(&mut self) {
        if self.caret > 0 {
            let prev = previous_boundary(&self.buffer, self.caret);
            self.buffer.replace_range(prev..self.caret, "");
            self.caret = prev;
        }
    }

    fn delete_forward(&mut self) {
        if self.caret < self.buffer.len() {
            let next = next_boundary(&self.buffer, self.caret);
            self.buffer.replace_range(self.caret..next, "");
        }
    }

    fn render_with_cursor(&self) -> String {
        format!(
            "{}|{}",
            &self.buffer[..self.caret],
            &self.buffer[self.caret..]
        )
    }
}

/// Links an editor entity to its parent control row entity.
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneEditOwner(pub Entity);

/// Marker for the Text entity that displays the editable value.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneEditableDisplay;

/// Number editing mode (digits, dot, minus; arrow up/down to step).
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneNumberEdit {
    pub step: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Plain text editing mode.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneTextEdit;

/// Hex color editing mode (hex digits + #).
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneHexEdit;

/// Slider value editing mode (number input that commits to SliderControl + SliderValue).
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneSliderEdit {
    pub step: f64,
    pub min: f64,
    pub max: f64,
}

// ── Systems ─────────────────────────────────────────────────

/// System (Update): set InputFocus when user clicks an editable control.
pub(crate) fn handle_pane_edit_focus(
    q_editable: Query<(Entity, &Interaction), (Changed<Interaction>, With<PaneEditState>)>,
    focus: Option<ResMut<InputFocus>>,
) {
    let Some(mut focus) = focus else { return };
    for (entity, interaction) in &q_editable {
        if *interaction == Interaction::Pressed {
            focus.0 = Some(entity);
        }
    }
}

/// System (Update): when color swatch is clicked, focus the sibling hex editor.
/// Disabled when ColorPickerOpen is present (swatch click opens picker instead).
pub(crate) fn handle_swatch_click(
    q_swatch: Query<(&Interaction, &ChildOf), (Changed<Interaction>, With<ColorSwatch>)>,
    q_children: Query<&Children>,
    q_editor: Query<Entity, With<PaneHexEdit>>,
    q_parent: Query<&ChildOf>,
    q_has_picker: Query<(), With<super::color_picker::ColorPickerOpen>>,
    focus: Option<ResMut<InputFocus>>,
) {
    let Some(mut focus) = focus else { return };
    for (interaction, child_of) in &q_swatch {
        if *interaction == Interaction::Pressed {
            let area_entity = child_of.parent();
            // If the row has ColorPickerOpen, swatch click opens picker — skip hex focus
            if let Ok(area_parent) = q_parent.get(area_entity) {
                if q_has_picker.contains(area_parent.parent()) {
                    continue;
                }
            }
            if let Ok(children) = q_children.get(area_entity) {
                for child in children.iter() {
                    if q_editor.contains(child) {
                        focus.0 = Some(child);
                        break;
                    }
                }
            }
        }
    }
}

/// Observer: handle keyboard input on focused editable pane controls.
pub(crate) fn on_pane_edit_keyboard(
    mut ev: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut q_editor: Query<(
        &mut PaneEditState,
        Option<&PaneNumberEdit>,
        Option<&PaneHexEdit>,
        Option<&PaneSliderEdit>,
    )>,
) {
    let Ok((mut state, number_edit, hex_edit, slider_edit)) = q_editor.get_mut(ev.focused_entity)
    else {
        return;
    };

    if ev.input.state != ButtonState::Pressed {
        return;
    }

    let _shortcut = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);

    match (&ev.input.logical_key, &ev.input.text) {
        (Key::ArrowLeft, _) => {
            ev.propagate(false);
            state.move_left();
        }
        (Key::ArrowRight, _) => {
            ev.propagate(false);
            state.move_right();
        }
        (Key::Home, _) => {
            ev.propagate(false);
            state.caret = 0;
        }
        (Key::End, _) => {
            ev.propagate(false);
            state.caret = state.buffer.len();
        }
        (Key::Backspace, _) => {
            ev.propagate(false);
            state.delete_backward();
        }
        (Key::Delete, _) => {
            ev.propagate(false);
            state.delete_forward();
        }
        // Number/Slider: arrow up/down to step value
        (Key::ArrowUp, _) if number_edit.is_some() || slider_edit.is_some() => {
            ev.propagate(false);
            let (step, min, max) = if let Some(ne) = number_edit {
                (ne.step, ne.min, ne.max)
            } else {
                let se = slider_edit.unwrap();
                (se.step, Some(se.min), Some(se.max))
            };
            let mut val = state.buffer.parse::<f64>().unwrap_or(0.0) + step;
            if let Some(mx) = max {
                val = val.min(mx);
            }
            if let Some(mn) = min {
                val = val.max(mn);
            }
            state.buffer = format_value(val);
            state.caret = state.buffer.len();
        }
        (Key::ArrowDown, _) if number_edit.is_some() || slider_edit.is_some() => {
            ev.propagate(false);
            let (step, min, max) = if let Some(ne) = number_edit {
                (ne.step, ne.min, ne.max)
            } else {
                let se = slider_edit.unwrap();
                (se.step, Some(se.min), Some(se.max))
            };
            let mut val = state.buffer.parse::<f64>().unwrap_or(0.0) - step;
            if let Some(mx) = max {
                val = val.min(mx);
            }
            if let Some(mn) = min {
                val = val.max(mn);
            }
            state.buffer = format_value(val);
            state.caret = state.buffer.len();
        }
        // Enter: commit and blur
        (Key::Enter, _) => {
            ev.propagate(false);
            commands.insert_resource(InputFocus(None));
        }
        // Escape: revert and blur
        (Key::Escape, _) => {
            ev.propagate(false);
            let orig = state.original.clone();
            state.buffer = orig;
            state.caret = state.buffer.len();
            commands.insert_resource(InputFocus(None));
        }
        // Character input: filter by mode
        (_, Some(text)) => {
            let filtered = if number_edit.is_some() || slider_edit.is_some() {
                filter_numeric(text)
            } else if hex_edit.is_some() {
                filter_hex(text)
            } else {
                filter_printable(text)
            };
            if !filtered.is_empty() {
                ev.propagate(false);
                state.insert_text(&filtered);
            }
        }
        _ => {}
    }
}

/// System (PostUpdate::Interaction): sync editing display, init on focus gain,
/// commit values on focus loss.
pub(crate) fn sync_pane_editing(
    focus: Option<Res<InputFocus>>,
    mut q_editors: Query<(
        Entity,
        &mut PaneEditState,
        &PaneEditOwner,
        Option<&PaneNumberEdit>,
        Option<&PaneHexEdit>,
        Option<&PaneSliderEdit>,
    )>,
    q_children: Query<&Children>,
    mut q_text: Query<&mut Text, With<PaneEditableDisplay>>,
    mut q_number: Query<&mut NumberControl>,
    mut q_text_ctrl: Query<&mut TextControl, Without<NumberControl>>,
    mut q_color: Query<&mut ColorControl, (Without<NumberControl>, Without<TextControl>)>,
    mut q_slider_ctrl: Query<
        &mut SliderControl,
        (
            Without<NumberControl>,
            Without<TextControl>,
            Without<ColorControl>,
        ),
    >,
    q_slider_link: Query<&SliderWidgetLink>,
    mut q_swatch: Query<&mut InlineStyle, With<ColorSwatch>>,
    mut q_classes: Query<&mut ClassList>,
    q_meta: Query<&PaneControlMeta>,
    mut commands: Commands,
    mut prev_focus: Local<Option<Entity>>,
) {
    let focused = focus.as_ref().and_then(|f| f.0);
    let focus_changed = focus.as_ref().is_some_and(|f| f.is_changed());
    let old_focus = *prev_focus;
    if focus_changed {
        *prev_focus = focused;
    }

    for (entity, mut state, owner, number_edit, hex_edit, slider_edit) in &mut q_editors {
        let is_focused = focused == Some(entity);
        let was_focused = old_focus == Some(entity);

        // ── Commit on blur ──
        if focus_changed && was_focused && !is_focused {
            let committed = state.buffer != state.original;

            if let Some(ne) = number_edit {
                if let Ok(mut ctrl) = q_number.get_mut(owner.0) {
                    if let Ok(val) = state.buffer.parse::<f64>() {
                        ctrl.value = clamp_number(val, ne);
                    }
                }
            } else if let Some(se) = slider_edit {
                if let Ok(mut ctrl) = q_slider_ctrl.get_mut(owner.0) {
                    if let Ok(val) = state.buffer.parse::<f64>() {
                        let val = val.clamp(se.min, se.max);
                        ctrl.value = val;
                        if let Ok(link) = q_slider_link.get(owner.0) {
                            commands.entity(link.0).insert(SliderValue(val as f32));
                        }
                    }
                }
            } else if hex_edit.is_some() {
                if let Ok(mut ctrl) = q_color.get_mut(owner.0) {
                    if let Some(color) = parse_hex_color(&state.buffer) {
                        ctrl.value = color;
                    }
                }
            } else if let Ok(mut ctrl) = q_text_ctrl.get_mut(owner.0) {
                ctrl.value.clone_from(&state.buffer);
            }

            // Emit PaneEditEnd event
            if let Ok(meta) = q_meta.get(owner.0) {
                let value = if number_edit.is_some() || slider_edit.is_some() {
                    PaneValue::Float(state.buffer.parse::<f64>().unwrap_or(0.0))
                } else if hex_edit.is_some() {
                    parse_hex_color(&state.buffer)
                        .map(PaneValue::Color)
                        .unwrap_or(PaneValue::String(state.buffer.clone()))
                } else {
                    PaneValue::String(state.buffer.clone())
                };
                commands.trigger(PaneEditEnd {
                    pane: meta.pane_title.clone(),
                    field: meta.label.clone(),
                    value,
                    committed,
                });
            }
        }

        // ── Initialize on focus gain ──
        if focus_changed && is_focused && !was_focused {
            // Emit PaneEditStart event
            if let Ok(meta) = q_meta.get(owner.0) {
                commands.trigger(PaneEditStart {
                    pane: meta.pane_title.clone(),
                    field: meta.label.clone(),
                });
            }

            let init_val = if number_edit.is_some() {
                q_number.get(owner.0).ok().map(|c| format_value(c.value))
            } else if slider_edit.is_some() {
                q_slider_ctrl
                    .get(owner.0)
                    .ok()
                    .map(|c| format_value(c.value))
            } else if hex_edit.is_some() {
                q_color.get(owner.0).ok().map(|c| color_to_hex(c.value))
            } else {
                q_text_ctrl.get(owner.0).ok().map(|c| c.value.clone())
            };
            if let Some(val) = init_val {
                state.buffer = val.clone();
                state.original = val;
                state.caret = state.buffer.len();
            }
        }

        // ── Update display text ──
        let display = if is_focused {
            state.render_with_cursor()
        } else if number_edit.is_some() {
            match q_number.get(owner.0) {
                Ok(ctrl) => format_value(ctrl.value),
                Err(_) => state.buffer.clone(),
            }
        } else if slider_edit.is_some() {
            match q_slider_ctrl.get(owner.0) {
                Ok(ctrl) => format_value(ctrl.value),
                Err(_) => state.buffer.clone(),
            }
        } else if hex_edit.is_some() {
            match q_color.get(owner.0) {
                Ok(ctrl) => color_to_hex(ctrl.value),
                Err(_) => state.buffer.clone(),
            }
        } else {
            match q_text_ctrl.get(owner.0) {
                Ok(ctrl) => ctrl.value.clone(),
                Err(_) => state.buffer.clone(),
            }
        };

        for desc in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_text.get_mut(desc) {
                if text.0 != display {
                    text.0 = display;
                }
                break;
            }
        }

        // ── Live swatch sync for hex color editing ──
        if is_focused && hex_edit.is_some() {
            if let Some(color) = parse_hex_color(&state.buffer) {
                let color_str = css_color(color);
                // Walk up to owner row, then find swatch descendant
                for desc in q_children.iter_descendants(owner.0) {
                    if let Ok(mut style) = q_swatch.get_mut(desc) {
                        style.set("--swatch-color", color_str.clone());
                        break;
                    }
                }
            }
        }

        // ── Update CSS class ──
        if let Ok(mut classes) = q_classes.get_mut(entity) {
            if is_focused {
                classes.add("is-focused");
            } else {
                classes.remove("is-focused");
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────

fn clamp_number(val: f64, edit: &PaneNumberEdit) -> f64 {
    let mut v = val;
    if let Some(min) = edit.min {
        v = v.max(min);
    }
    if let Some(max) = edit.max {
        v = v.min(max);
    }
    v
}

pub(crate) fn format_value(value: f64) -> String {
    format!("{:.2}", value)
}

pub(crate) fn color_to_hex(color: Color) -> String {
    let srgba = color.to_srgba();
    let r = (srgba.red * 255.0).round() as u8;
    let g = (srgba.green * 255.0).round() as u8;
    let b = (srgba.blue * 255.0).round() as u8;
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::srgb(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
    ))
}

fn filter_numeric(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_digit() || matches!(*c, '.' | '-' | '+'))
        .collect()
}

fn filter_hex(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_hexdigit() || *c == '#')
        .collect()
}

fn filter_printable(text: &str) -> String {
    text.chars().filter(|c| !c.is_ascii_control()).collect()
}

fn previous_boundary(text: &str, index: usize) -> usize {
    if index == 0 {
        return 0;
    }
    let mut prev = 0;
    for (byte_index, _) in text.char_indices() {
        if byte_index >= index {
            break;
        }
        prev = byte_index;
    }
    prev
}

fn next_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    for (byte_index, _) in text.char_indices() {
        if byte_index > index {
            return byte_index;
        }
    }
    text.len()
}
