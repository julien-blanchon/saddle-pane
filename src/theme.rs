use bevy::prelude::*;
use bevy_flair::prelude::InlineStyle;

use crate::layout::PaneRoot;

/// Global pane theme resource. Change this to switch all panes' color scheme.
#[derive(Resource, Clone, Debug)]
pub struct PaneTheme {
    pub active: PaneThemePreset,
}

impl Default for PaneTheme {
    fn default() -> Self {
        Self {
            active: PaneThemePreset::Dark,
        }
    }
}

impl PaneTheme {
    /// Cycle between Dark and Light themes.
    pub fn toggle(&mut self) {
        self.active = match self.active {
            PaneThemePreset::Dark => PaneThemePreset::Light,
            PaneThemePreset::Light => PaneThemePreset::Dark,
        };
    }
}

/// Available theme presets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PaneThemePreset {
    #[default]
    Dark,
    Light,
}

/// Per-pane theme override. When present, this pane ignores the global `PaneTheme`.
#[derive(Component, Clone, Debug)]
pub struct PaneThemeOverride(pub PaneThemePreset);

/// Returns CSS variable overrides for the given theme preset.
/// For Dark, returns empty (defaults in pane.css are the dark theme).
/// For Light, returns a full set of light-mode overrides.
fn theme_vars(preset: PaneThemePreset) -> Vec<(&'static str, String)> {
    match preset {
        PaneThemePreset::Dark => vec![],
        PaneThemePreset::Light => vec![
            // Surface colors
            ("--pane-elevation-1", "#f0f0f2".into()),
            ("--pane-elevation-2", "#e8e8ec".into()),
            ("--pane-elevation-3", "rgba(0, 0, 0, 0.06)".into()),
            // Border colors
            ("--pane-border", "#c8c9ce".into()),
            ("--pane-border-focus", "#4a7ab5".into()),
            ("--pane-border-subtle", "#d8d8de".into()),
            // Text colors
            ("--pane-text-primary", "#2a2b30".into()),
            ("--pane-text-secondary", "#6a6b72".into()),
            ("--pane-text-muted", "#9a9ba2".into()),
            ("--pane-text-on-accent", "#ffffff".into()),
            ("--pane-text-brighter", "#1a1b20".into()),
            ("--pane-text-monitor", "#6a6b72".into()),
            ("--pane-text-log", "#7a7b82".into()),
            // Accent colors
            ("--pane-accent", "#3a6db5".into()),
            ("--pane-accent-hover", "#2a5da5".into()),
            ("--pane-accent-active", "#1a4d95".into()),
            ("--pane-accent-subtle", "rgba(58, 109, 181, 0.10)".into()),
            // Accent fill variants
            ("--pane-accent-fill", "rgba(58, 109, 181, 0.50)".into()),
            ("--pane-accent-fill-hover", "rgba(42, 93, 165, 0.60)".into()),
            ("--pane-accent-fill-active", "rgba(42, 93, 165, 0.70)".into()),
            ("--pane-accent-checked", "rgba(58, 109, 181, 0.20)".into()),
            ("--pane-accent-checked-hover", "rgba(58, 109, 181, 0.30)".into()),
            ("--pane-accent-indicator", "rgba(58, 109, 181, 0.70)".into()),
            ("--pane-accent-knob", "#3a6db5".into()),
            // Widget backgrounds
            ("--pane-widget-bg", "rgba(0, 0, 0, 0.06)".into()),
            ("--pane-widget-hover", "rgba(0, 0, 0, 0.09)".into()),
            ("--pane-widget-focus", "rgba(0, 0, 0, 0.12)".into()),
            ("--pane-widget-active", "rgba(0, 0, 0, 0.16)".into()),
            ("--pane-widget-bg-muted", "rgba(0, 0, 0, 0.03)".into()),
            ("--pane-tab-hover-bg", "rgba(0, 0, 0, 0.04)".into()),
            // Interactive states
            ("--pane-hover-bg", "rgba(0, 0, 0, 0.03)".into()),
            ("--pane-active-bg", "rgba(0, 0, 0, 0.05)".into()),
            // Popup / dark backgrounds
            ("--pane-popup-bg", "#ffffff".into()),
            ("--pane-bg-dark", "rgba(0, 0, 0, 0.08)".into()),
        ],
    }
}

/// System: apply theme CSS variable overrides when `PaneTheme` changes.
pub(crate) fn apply_theme(
    theme: Res<PaneTheme>,
    mut q_panes: Query<
        &mut InlineStyle,
        (With<PaneRoot>, Without<PaneThemeOverride>),
    >,
) {
    if !theme.is_changed() {
        return;
    }
    let vars = theme_vars(theme.active);
    for mut style in &mut q_panes {
        apply_vars(&mut style, &vars);
    }
}

/// System: apply per-pane theme overrides for panes with `PaneThemeOverride`.
pub(crate) fn apply_pane_theme_override(
    mut q_panes: Query<
        (&PaneThemeOverride, &mut InlineStyle),
        (With<PaneRoot>, Changed<PaneThemeOverride>),
    >,
) {
    for (override_theme, mut style) in &mut q_panes {
        let vars = theme_vars(override_theme.0);
        apply_vars(&mut style, &vars);
    }
}

fn apply_vars(style: &mut InlineStyle, vars: &[(&str, String)]) {
    if vars.is_empty() {
        // Dark theme: InlineStyle has no "remove" API, so we explicitly set
        // dark values to undo any light-theme overrides from a previous switch.
        let dark_vars = [
            ("--pane-elevation-1", "#28292e"),
            ("--pane-elevation-2", "#222327"),
            ("--pane-elevation-3", "rgba(187, 188, 196, 0.10)"),
            ("--pane-border", "#3c3d44"),
            ("--pane-border-focus", "#7090b0"),
            ("--pane-border-subtle", "#333438"),
            ("--pane-text-primary", "#bbbcc4"),
            ("--pane-text-secondary", "#78797f"),
            ("--pane-text-muted", "#5c5d64"),
            ("--pane-text-on-accent", "#ffffff"),
            ("--pane-text-brighter", "#d0d1d8"),
            ("--pane-text-monitor", "#9a9ba2"),
            ("--pane-text-log", "#8a8b92"),
            ("--pane-accent", "#4a6fa5"),
            ("--pane-accent-hover", "#5a8fd5"),
            ("--pane-accent-active", "#3a5f95"),
            ("--pane-accent-subtle", "rgba(74, 111, 165, 0.15)"),
            ("--pane-accent-fill", "rgba(74, 111, 165, 0.60)"),
            ("--pane-accent-fill-hover", "rgba(90, 143, 213, 0.70)"),
            ("--pane-accent-fill-active", "rgba(90, 143, 213, 0.80)"),
            ("--pane-accent-checked", "rgba(74, 111, 165, 0.25)"),
            ("--pane-accent-checked-hover", "rgba(74, 111, 165, 0.35)"),
            ("--pane-accent-indicator", "rgba(74, 111, 165, 0.80)"),
            ("--pane-accent-knob", "#7aacdf"),
            ("--pane-widget-bg", "rgba(187, 188, 196, 0.10)"),
            ("--pane-widget-hover", "rgba(187, 188, 196, 0.15)"),
            ("--pane-widget-focus", "rgba(187, 188, 196, 0.20)"),
            ("--pane-widget-active", "rgba(187, 188, 196, 0.25)"),
            ("--pane-widget-bg-muted", "rgba(187, 188, 196, 0.06)"),
            ("--pane-tab-hover-bg", "rgba(187, 188, 196, 0.06)"),
            ("--pane-hover-bg", "rgba(255, 255, 255, 0.03)"),
            ("--pane-active-bg", "rgba(255, 255, 255, 0.05)"),
            ("--pane-popup-bg", "#1e1f24"),
            ("--pane-bg-dark", "rgba(0, 0, 0, 0.25)"),
        ];
        for &(key, val) in &dark_vars {
            style.set(key, val.to_string());
        }
    } else {
        for (key, val) in vars {
            style.set(*key, val.clone());
        }
    }
}
