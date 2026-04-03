pub mod button;
pub mod color;
pub mod color_picker;
pub mod editing;
pub mod monitor;
pub mod number;
pub(crate) mod scroll;
pub mod select;
pub mod separator;
pub mod slider;
pub mod text;
pub mod toggle;

use bevy::prelude::*;
use bevy::text::TextFont;
use bevy::ui::Checked;
use bevy_flair::prelude::ClassList;
use bevy_ui_widgets::SliderValue;

use self::color::ColorControl;
use self::number::NumberControl;
use self::select::SelectControl;
use self::slider::{SliderControl, SliderWidgetLink};
use self::text::TextControl;
use self::toggle::{ToggleControl, ToggleWidgetLink};
use crate::events::PaneButtonPressed;
use crate::registry::CustomValueBox;

/// Type-erased value for the pane store and events.
#[derive(Clone, Debug, PartialEq)]
pub enum PaneValue {
    Float(f64),
    Bool(bool),
    String(String),
    Color(Color),
    Int(i64),
    /// Custom value from a plugin control. Downcast via `.0.as_any().downcast_ref::<T>()`.
    Custom(CustomValueBox),
}

/// Metadata shared by all pane controls.
#[derive(Component, Clone, Debug)]
pub struct PaneControlMeta {
    pub pane_title: String,
    pub label: String,
    pub tooltip: Option<String>,
    pub order: i32,
    /// Optional SVG icon string (from `bevy_iconify::svg!(...)` or `saddle_pane::icons::ICON_*`).
    pub icon: Option<String>,
    /// Pre-rasterized icon image handle (set internally by the layout system).
    #[doc(hidden)]
    pub icon_handle: Option<Handle<Image>>,
}

/// Stores the initial value of a control for reset support.
#[derive(Component, Clone, Debug)]
pub struct InitialValue(pub PaneValue);

/// Helper: convert a `Color` to a CSS color string `rgb(R G B / A)`.
pub(crate) fn css_color(color: Color) -> String {
    let linear = color.to_linear();
    let r = (linear.red * 255.0).round() as u8;
    let g = (linear.green * 255.0).round() as u8;
    let b = (linear.blue * 255.0).round() as u8;
    let a = linear.alpha;
    if (a - 1.0).abs() < f32::EPSILON {
        format!("rgb({r}, {g}, {b})")
    } else {
        format!("rgba({r}, {g}, {b}, {a:.2})")
    }
}

/// Helper: format a percentage for CSS. Public for plugin authors.
pub fn css_percent(value: f32) -> String {
    format!("{:.4}%", value.clamp(0.0, 100.0))
}

/// Standard small font for pane labels and values. Public for plugin authors.
pub fn pane_font(size: f32) -> TextFont {
    TextFont {
        font_size: size,
        ..default()
    }
}

/// Default label font.
pub(crate) fn label_font() -> TextFont {
    pane_font(11.0)
}

/// Default value font.
pub(crate) fn value_font() -> TextFont {
    pane_font(11.0)
}

/// Spawn a label as a Node container wrapping a Text child. Public for plugin authors.
pub fn spawn_label(parent: &mut ChildSpawnerCommands, text: &str) {
    spawn_label_with_icon(parent, text, None, None);
}

/// Spawn a label with an optional SVG icon prefix. Public for plugin authors.
///
/// When a pre-resolved icon handle is available (via `meta.icon_handle`), the icon is
/// rendered immediately with `UiVelloSvg`. Otherwise falls back to `PaneIconPlaceholder`
/// for deferred resolution.
pub fn spawn_label_with_icon(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    icon: Option<&str>,
    icon_handle: Option<Handle<Image>>,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(40.0),
                min_width: Val::Px(0.0),
                flex_shrink: 0.0,
                flex_grow: 0.0,
                display: Display::Flex,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(4.0), Val::ZERO),
                overflow: Overflow::clip(),
                column_gap: Val::Px(3.0),
                ..default()
            },
            ClassList::new("pane-label"),
        ))
        .with_children(|label| {
            if let Some(handle) = icon_handle {
                // Pre-resolved handle — spawn UiVelloSvg directly
                crate::icons::spawn_pane_icon_handle(label, handle, 12.0);
            } else if let Some(svg) = icon {
                // Fallback — deferred resolution via placeholder
                crate::icons::spawn_pane_icon(label, svg, 12.0);
            }
            label.spawn((
                Text::new(text),
                label_font(),
                ClassList::new("pane-label-text"),
            ));
        });
}

/// Observer: handle "Reset All" (or similar) button press by resetting controls.
pub(crate) fn on_pane_reset_button(
    ev: On<PaneButtonPressed>,
    mut q_sliders: Query<(
        &PaneControlMeta,
        &InitialValue,
        &mut SliderControl,
        &SliderWidgetLink,
    )>,
    mut q_toggles: Query<
        (
            &PaneControlMeta,
            &InitialValue,
            &mut ToggleControl,
            &ToggleWidgetLink,
        ),
        Without<SliderControl>,
    >,
    mut q_numbers: Query<
        (&PaneControlMeta, &InitialValue, &mut NumberControl),
        (Without<SliderControl>, Without<ToggleControl>),
    >,
    mut q_texts: Query<
        (&PaneControlMeta, &InitialValue, &mut TextControl),
        (
            Without<SliderControl>,
            Without<ToggleControl>,
            Without<NumberControl>,
        ),
    >,
    mut q_selects: Query<
        (&PaneControlMeta, &InitialValue, &mut SelectControl),
        (
            Without<SliderControl>,
            Without<ToggleControl>,
            Without<NumberControl>,
            Without<TextControl>,
        ),
    >,
    mut q_colors: Query<
        (&PaneControlMeta, &InitialValue, &mut ColorControl),
        (
            Without<SliderControl>,
            Without<ToggleControl>,
            Without<NumberControl>,
            Without<TextControl>,
            Without<SelectControl>,
        ),
    >,
    mut commands: Commands,
) {
    // Only handle buttons whose label contains "Reset"
    if !ev.event().label.to_lowercase().contains("reset") {
        return;
    }
    let pane = &ev.event().pane;

    for (meta, init, mut ctrl, link) in &mut q_sliders {
        if &meta.pane_title == pane {
            if let PaneValue::Float(v) = &init.0 {
                ctrl.value = *v;
                commands.entity(link.0).insert(SliderValue(*v as f32));
            }
        }
    }
    for (meta, init, mut ctrl, link) in &mut q_toggles {
        if &meta.pane_title == pane {
            if let PaneValue::Bool(v) = &init.0 {
                ctrl.value = *v;
                if *v {
                    commands.entity(link.0).insert(Checked);
                } else {
                    commands.entity(link.0).remove::<Checked>();
                }
            }
        }
    }
    for (meta, init, mut ctrl) in &mut q_numbers {
        if &meta.pane_title == pane {
            if let PaneValue::Float(v) = &init.0 {
                ctrl.value = *v;
            }
        }
    }
    for (meta, init, mut ctrl) in &mut q_texts {
        if &meta.pane_title == pane {
            if let PaneValue::String(v) = &init.0 {
                ctrl.value.clone_from(v);
            }
        }
    }
    for (meta, init, mut ctrl) in &mut q_selects {
        if &meta.pane_title == pane {
            if let PaneValue::Int(v) = &init.0 {
                ctrl.value = *v as usize;
            }
        }
    }
    for (meta, init, mut ctrl) in &mut q_colors {
        if &meta.pane_title == pane {
            if let PaneValue::Color(v) = &init.0 {
                ctrl.value = *v;
            }
        }
    }
}
