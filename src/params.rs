//! Control configuration structs for the builder API.
//!
//! Each complex control type has a config struct with a fluent builder.
//! Simple controls (toggle, text, color, button) can be used with plain args
//! or with an optional config struct for advanced options.

use std::ops::RangeInclusive;

use bevy::prelude::Color;

/// Shared optional properties that any control can have.
#[derive(Clone, Debug, Default)]
pub(crate) struct ControlOpts {
    pub tooltip: Option<String>,
    pub icon: Option<String>,
}

// ── Slider ──

/// Configuration for a slider control.
///
/// ```rust,ignore
/// .slider("Speed", Slider::new(0.0..=10.0, 5.0).step(0.1).tooltip("Units/sec"))
/// ```
#[derive(Clone, Debug)]
pub struct Slider {
    pub(crate) min: f64,
    pub(crate) max: f64,
    pub(crate) default: f64,
    pub(crate) step: f64,
    pub(crate) opts: ControlOpts,
}

impl Slider {
    /// Create a slider with a range and default value.
    pub fn new(range: RangeInclusive<f64>, default: f64) -> Self {
        Self {
            min: *range.start(),
            max: *range.end(),
            default,
            step: 0.01,
            opts: ControlOpts::default(),
        }
    }

    /// Set the step increment (default: 0.01).
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── Number ──

/// Configuration for a number input control.
///
/// ```rust,ignore
/// .number("Score", Number::new(100.0).step(1.0).min(0.0).max(999.0))
/// ```
#[derive(Clone, Debug)]
pub struct Number {
    pub(crate) default: f64,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) step: f64,
    pub(crate) step_buttons: bool,
    pub(crate) opts: ControlOpts,
}

impl Number {
    /// Create a number input with a default value.
    pub fn new(default: f64) -> Self {
        Self {
            default,
            min: None,
            max: None,
            step: 1.0,
            step_buttons: true,
            opts: ControlOpts::default(),
        }
    }

    /// Set the step increment (default: 1.0).
    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
        self
    }

    /// Set minimum allowed value.
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum allowed value.
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Show or hide +/- step buttons (default: true).
    pub fn step_buttons(mut self, show: bool) -> Self {
        self.step_buttons = show;
        self
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── Toggle (optional config) ──

/// Optional configuration for a toggle control.
///
/// ```rust,ignore
/// .toggle_opts("Debug", Toggle::new(false).tooltip("Show overlays").icon(ICON_BUG))
/// ```
#[derive(Clone, Debug)]
pub struct Toggle {
    pub(crate) default: bool,
    pub(crate) opts: ControlOpts,
}

impl Toggle {
    /// Create a toggle config with a default value.
    pub fn new(default: bool) -> Self {
        Self {
            default,
            opts: ControlOpts::default(),
        }
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── Button (optional config) ──

/// Optional configuration for a button control.
///
/// ```rust,ignore
/// .button_opts("Save", Button::new().tooltip("Save to disk").icon(ICON_SAVE))
/// ```
#[derive(Clone, Debug, Default)]
pub struct ButtonOpts {
    pub(crate) opts: ControlOpts,
}

impl ButtonOpts {
    /// Create a button config.
    pub fn new() -> Self {
        Self {
            opts: ControlOpts::default(),
        }
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── ColorPicker (optional config) ──

/// Optional configuration for a color picker control.
///
/// ```rust,ignore
/// .color_opts("Ambient", ColorPicker::new(Color::WHITE).tooltip("Scene ambient"))
/// ```
#[derive(Clone, Debug)]
pub struct ColorPicker {
    pub(crate) default: Color,
    pub(crate) opts: ControlOpts,
}

impl ColorPicker {
    /// Create a color picker config with a default color.
    pub fn new(default: Color) -> Self {
        Self {
            default,
            opts: ControlOpts::default(),
        }
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── TextInput (optional config) ──

/// Optional configuration for a text input control.
///
/// ```rust,ignore
/// .text_opts("Name", TextInput::new("Hero").tooltip("Player name"))
/// ```
#[derive(Clone, Debug)]
pub struct TextInput {
    pub(crate) default: String,
    pub(crate) opts: ControlOpts,
}

impl TextInput {
    /// Create a text input config with a default value.
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            default: default.into(),
            opts: ControlOpts::default(),
        }
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── SelectMenu (optional config) ──

/// Optional configuration for a select/dropdown control.
///
/// ```rust,ignore
/// .select_opts("Quality", SelectMenu::new(&["Low", "Med", "High"], 1).tooltip("Render quality"))
/// ```
#[derive(Clone, Debug)]
pub struct SelectMenu {
    pub(crate) options: Vec<String>,
    pub(crate) default: usize,
    pub(crate) opts: ControlOpts,
}

impl SelectMenu {
    /// Create a select config with options and default index.
    pub fn new(options: &[&str], default: usize) -> Self {
        Self {
            options: options.iter().map(|s| s.to_string()).collect(),
            default,
            opts: ControlOpts::default(),
        }
    }

    /// Set hover tooltip text.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.opts.tooltip = Some(text.into());
        self
    }

    /// Set an SVG icon for the label.
    pub fn icon(mut self, svg: &str) -> Self {
        self.opts.icon = Some(svg.to_string());
        self
    }
}

// ── Monitor ──

/// Configuration for a read-only monitor control.
///
/// ```rust,ignore
/// .monitor("FPS", Monitor::text("—"))
/// .monitor("Console", Monitor::log(8))
/// .monitor("CPU", Monitor::graph(0.0..=100.0, 64))
/// ```
#[derive(Clone, Debug)]
pub enum Monitor {
    /// Single-line text display.
    Text { default: String },
    /// Scrollable log buffer.
    Log { buffer_size: usize },
    /// Sparkline graph.
    Graph {
        min: f64,
        max: f64,
        buffer_size: usize,
    },
}

impl Monitor {
    /// Create a single-line text monitor.
    pub fn text(default: impl Into<String>) -> Self {
        Self::Text {
            default: default.into(),
        }
    }

    /// Create a scrollable log monitor.
    pub fn log(buffer_size: usize) -> Self {
        Self::Log { buffer_size }
    }

    /// Create a sparkline graph monitor.
    pub fn graph(range: RangeInclusive<f64>, buffer_size: usize) -> Self {
        Self::Graph {
            min: *range.start(),
            max: *range.end(),
            buffer_size,
        }
    }
}
