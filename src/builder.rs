use std::ops::RangeInclusive;

use bevy::prelude::*;

use crate::layout;
use crate::params::{
    ButtonOpts, ColorPicker, Monitor, Number, SelectMenu, Slider, TextInput, Toggle,
};
use crate::registry::ControlConfig;

/// Specification for a single control (internal).
#[derive(Clone, Debug)]
pub(crate) enum ControlSpec {
    Slider {
        label: String,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Toggle {
        label: String,
        default: bool,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Button {
        label: String,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Number {
        label: String,
        default: f64,
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
        step_buttons: bool,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Text {
        label: String,
        default: String,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Select {
        label: String,
        options: Vec<String>,
        default: usize,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Color {
        label: String,
        default: Color,
        tooltip: Option<String>,
        icon: Option<String>,
    },
    Monitor {
        label: String,
        default: String,
    },
    MonitorLog {
        label: String,
        buffer_size: usize,
    },
    MonitorGraph {
        label: String,
        min: f64,
        max: f64,
        buffer_size: usize,
    },
    Separator,
    /// A custom control registered via the plugin system.
    Custom {
        control_id: String,
        label: String,
        config: ControlConfig,
        tooltip: Option<String>,
        icon: Option<String>,
    },
}

/// A layout item: control, folder, or tab group.
#[derive(Clone, Debug)]
pub(crate) enum LayoutItem {
    Control(ControlSpec),
    Folder {
        label: String,
        items: Vec<LayoutItem>,
        collapsed: bool,
    },
    TabGroup {
        tabs: Vec<TabSpec>,
        active: usize,
    },
}

/// Specification for a single tab page.
#[derive(Clone, Debug)]
pub(crate) struct TabSpec {
    pub label: String,
    pub items: Vec<LayoutItem>,
}

/// Predefined pane positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PanePosition {
    /// Absolute position from top-left corner.
    Absolute(f32, f32),
    /// Top-left corner with margin.
    TopLeft,
    /// Top-right corner with margin (default).
    TopRight,
    /// Bottom-left corner with margin.
    BottomLeft,
    /// Bottom-right corner with margin.
    BottomRight,
}

/// The final specification produced by `PaneBuilder::build()`.
#[derive(Clone, Debug)]
pub struct PaneSpec {
    pub(crate) title: String,
    pub(crate) items: Vec<LayoutItem>,
    pub(crate) footer: Vec<LayoutItem>,
    pub(crate) collapsed: bool,
    pub(crate) position: Option<PanePosition>,
    pub(crate) width: Option<f32>,
    pub(crate) searchable: bool,
}

// ── Macro: shared control methods for PaneBuilder / FolderBuilder ──

/// Generates all control methods on a builder type that has an `items: Vec<LayoutItem>` field.
macro_rules! impl_control_methods {
    () => {
        /// Add a slider control with full configuration.
        pub fn slider(mut self, label: &str, config: Slider) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Slider {
                label: label.to_string(),
                min: config.min,
                max: config.max,
                step: config.step,
                default: config.default,
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a toggle (boolean) control.
        pub fn toggle(mut self, label: &str, default: bool) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Toggle {
                label: label.to_string(),
                default,
                tooltip: None,
                icon: None,
            }));
            self
        }

        /// Add a toggle with full configuration.
        pub fn toggle_opts(mut self, label: &str, config: Toggle) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Toggle {
                label: label.to_string(),
                default: config.default,
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a button control.
        pub fn button(mut self, label: &str) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Button {
                label: label.to_string(),
                tooltip: None,
                icon: None,
            }));
            self
        }

        /// Add a button with full configuration.
        pub fn button_opts(mut self, label: &str, config: ButtonOpts) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Button {
                label: label.to_string(),
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a number input control with full configuration.
        pub fn number(mut self, label: &str, config: Number) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Number {
                label: label.to_string(),
                default: config.default,
                min: config.min,
                max: config.max,
                step: config.step,
                step_buttons: config.step_buttons,
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a text input control.
        pub fn text(mut self, label: &str, default: &str) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Text {
                label: label.to_string(),
                default: default.to_string(),
                tooltip: None,
                icon: None,
            }));
            self
        }

        /// Add a text input with full configuration.
        pub fn text_opts(mut self, label: &str, config: TextInput) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Text {
                label: label.to_string(),
                default: config.default,
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a select/dropdown control.
        pub fn select(mut self, label: &str, options: &[&str], default: usize) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Select {
                label: label.to_string(),
                options: options.iter().map(|s| s.to_string()).collect(),
                default,
                tooltip: None,
                icon: None,
            }));
            self
        }

        /// Add a select/dropdown with full configuration.
        pub fn select_opts(mut self, label: &str, config: SelectMenu) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Select {
                label: label.to_string(),
                options: config.options,
                default: config.default,
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a color picker control.
        pub fn color(mut self, label: &str, default: Color) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Color {
                label: label.to_string(),
                default,
                tooltip: None,
                icon: None,
            }));
            self
        }

        /// Add a color picker with full configuration.
        pub fn color_opts(mut self, label: &str, config: ColorPicker) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Color {
                label: label.to_string(),
                default: config.default,
                tooltip: config.opts.tooltip,
                icon: config.opts.icon,
            }));
            self
        }

        /// Add a read-only monitor control.
        pub fn monitor(mut self, label: &str, config: Monitor) -> Self {
            match config {
                Monitor::Text { default } => {
                    self.items.push(LayoutItem::Control(ControlSpec::Monitor {
                        label: label.to_string(),
                        default,
                    }));
                }
                Monitor::Log { buffer_size } => {
                    self.items
                        .push(LayoutItem::Control(ControlSpec::MonitorLog {
                            label: label.to_string(),
                            buffer_size,
                        }));
                }
                Monitor::Graph {
                    min,
                    max,
                    buffer_size,
                } => {
                    self.items
                        .push(LayoutItem::Control(ControlSpec::MonitorGraph {
                            label: label.to_string(),
                            min,
                            max,
                            buffer_size,
                        }));
                }
            }
            self
        }

        /// Add an interval (range) control (requires `PaneIntervalPlugin`).
        pub fn interval(
            self,
            label: &str,
            bounds: RangeInclusive<f64>,
            default: RangeInclusive<f64>,
        ) -> Self {
            let config = ControlConfig::new()
                .float("bounds_min", *bounds.start())
                .float("bounds_max", *bounds.end())
                .float("default_min", *default.start())
                .float("default_max", *default.end())
                .float("step", 0.01);
            self.custom("interval", label, config)
        }

        /// Add a custom control by plugin ID.
        pub fn custom(mut self, control_id: &str, label: &str, config: ControlConfig) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Custom {
                control_id: control_id.to_string(),
                label: label.to_string(),
                config,
                tooltip: None,
                icon: None,
            }));
            self
        }

        /// Add a visual separator line.
        pub fn separator(mut self) -> Self {
            self.items.push(LayoutItem::Control(ControlSpec::Separator));
            self
        }

        /// Add a collapsible folder with nested controls.
        pub fn folder(
            mut self,
            label: &str,
            f: impl FnOnce(FolderBuilder) -> FolderBuilder,
        ) -> Self {
            let folder = f(FolderBuilder::new());
            self.items.push(LayoutItem::Folder {
                label: label.to_string(),
                items: folder.items,
                collapsed: folder.collapsed,
            });
            self
        }
    };
}

// ── PaneBuilder ──

/// Fluent builder for creating debug panes.
pub struct PaneBuilder {
    title: String,
    items: Vec<LayoutItem>,
    footer: Vec<LayoutItem>,
    collapsed: bool,
    position: Option<PanePosition>,
    width: Option<f32>,
    searchable: bool,
}

impl PaneBuilder {
    /// Create a new pane builder with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            footer: Vec::new(),
            collapsed: false,
            position: None,
            width: None,
            searchable: false,
        }
    }

    impl_control_methods!();

    /// Add a tab page. Call multiple times for multiple tabs.
    ///
    /// ```rust,ignore
    /// PaneBuilder::new("Settings")
    ///     .tab("General", |p| p.slider("Speed", Slider::new(0.0..=10.0, 5.0)))
    ///     .tab("Physics", |p| p.slider("Gravity", Slider::new(-20.0..=0.0, -9.81)))
    ///     .footer(|f| f.button("Reset All"))
    ///     .spawn(&mut commands);
    /// ```
    pub fn tab(
        mut self,
        label: &str,
        f: impl FnOnce(FolderBuilder) -> FolderBuilder,
    ) -> Self {
        let page = f(FolderBuilder::new());
        // Find or create the tab group in items
        let tab_spec = TabSpec {
            label: label.to_string(),
            items: page.items,
        };
        // If the last item is a TabGroup, append to it; otherwise create a new one
        if let Some(LayoutItem::TabGroup { tabs, .. }) = self.items.last_mut() {
            tabs.push(tab_spec);
        } else {
            self.items.push(LayoutItem::TabGroup {
                tabs: vec![tab_spec],
                active: 0,
            });
        }
        self
    }

    /// Add a pinned footer section (renders below the scroll area).
    ///
    /// ```rust,ignore
    /// .footer(|f| f.button("Reset All").button("Save").button("Load"))
    /// ```
    pub fn footer(
        mut self,
        f: impl FnOnce(FolderBuilder) -> FolderBuilder,
    ) -> Self {
        let built = f(FolderBuilder::new());
        self.footer = built.items;
        self
    }

    // ── Pane config ──

    /// Set pane position (absolute, from top-left).
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = Some(PanePosition::Absolute(x, y));
        self
    }

    /// Place the pane at a predefined corner.
    pub fn at(mut self, pos: PanePosition) -> Self {
        self.position = Some(pos);
        self
    }

    /// Set pane width in pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Start the pane in collapsed state.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Enable the search/filter bar at the top of the pane.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Build the pane spec without spawning.
    pub fn build(self) -> PaneSpec {
        PaneSpec {
            title: self.title,
            items: self.items,
            footer: self.footer,
            collapsed: self.collapsed,
            position: self.position,
            width: self.width,
            searchable: self.searchable,
        }
    }

    /// Build and spawn the pane, returning the root entity.
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let spec = self.build();
        layout::spawn_pane(commands, spec)
    }
}

// ── FolderBuilder ──

/// Builder for folder / tab page / footer contents.
pub struct FolderBuilder {
    pub(crate) items: Vec<LayoutItem>,
    pub(crate) collapsed: bool,
}

impl FolderBuilder {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            collapsed: false,
        }
    }

    impl_control_methods!();

    /// Start the folder in collapsed state.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
