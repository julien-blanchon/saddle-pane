use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;

use super::{PaneControlMeta, label_font, pane_font};

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/monitor.css";

// ── Read-only value monitor ──

/// Read-only value display that updates from external data.
#[derive(Component, Clone, Debug)]
pub struct MonitorControl {
    pub value: String,
}

/// Marker on the monitor value text entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct MonitorValueText;

/// Spawn a read-only monitor display.
pub(crate) fn spawn_monitor_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &MonitorControl,
    asset_server: &AssetServer,
) -> Entity {
    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row pane-row-monitor"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            super::spawn_label_with_icon(
                row,
                &meta.label,
                meta.icon.as_deref(),
                meta.icon_handle.clone(),
            );

            row.spawn((Node::default(), ClassList::new("pane-monitor-value")))
                .with_children(|val| {
                    val.spawn((
                        Text::new(&control.value),
                        pane_font(10.0),
                        ClassList::new("pane-monitor-value-text"),
                        MonitorValueText,
                    ));
                });
        });

    row_entity
}

/// System: update monitor text when MonitorControl changes.
pub(crate) fn update_monitor_text(
    q_monitors: Query<(Entity, &MonitorControl), Changed<MonitorControl>>,
    q_children: Query<&Children>,
    mut q_text: Query<&mut Text, With<MonitorValueText>>,
) {
    for (entity, monitor) in &q_monitors {
        for descendant in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_text.get_mut(descendant) {
                text.0 = monitor.value.clone();
                break;
            }
        }
    }
}

// ── Multiline log monitor ──

/// Scrolling text log with configurable buffer size.
#[derive(Component, Clone, Debug)]
pub struct MonitorLogControl {
    pub lines: VecDeque<String>,
    pub buffer_size: usize,
}

impl MonitorLogControl {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(buffer_size),
            buffer_size,
        }
    }

    /// Push a new line, evicting the oldest if at capacity.
    pub fn push(&mut self, line: impl Into<String>) {
        if self.lines.len() >= self.buffer_size {
            self.lines.pop_front();
        }
        self.lines.push_back(line.into());
    }

    /// Clear all lines.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    fn display_text(&self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(line);
        }
        if out.is_empty() {
            out.push('\u{2014}');
        }
        out
    }
}

/// Marker on the log text entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct MonitorLogText;

/// Spawn a multiline log monitor.
pub(crate) fn spawn_monitor_log_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &MonitorLogControl,
    asset_server: &AssetServer,
) -> Entity {
    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row-log"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Label at top
            row.spawn((Node::default(), ClassList::new("pane-log-header")))
                .with_children(|header| {
                    header.spawn((
                        Text::new(&meta.label),
                        label_font(),
                        ClassList::new("pane-label-text"),
                    ));
                });

            // Scrolling log area
            row.spawn((Node::default(), ClassList::new("pane-log-area")))
                .with_children(|area| {
                    area.spawn((
                        Text::new(control.display_text()),
                        pane_font(9.0),
                        ClassList::new("pane-log-text"),
                        MonitorLogText,
                    ));
                });
        });

    row_entity
}

/// System: update log text when MonitorLogControl changes.
pub(crate) fn update_monitor_log_text(
    q_logs: Query<(Entity, &MonitorLogControl), Changed<MonitorLogControl>>,
    q_children: Query<&Children>,
    mut q_text: Query<&mut Text, With<MonitorLogText>>,
) {
    for (entity, log) in &q_logs {
        let display = log.display_text();
        for descendant in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_text.get_mut(descendant) {
                text.0 = display;
                break;
            }
        }
    }
}

// ── Graph monitor ──

/// Sparkline graph showing value history.
#[derive(Component, Clone, Debug)]
pub struct MonitorGraphControl {
    pub values: VecDeque<f32>,
    pub min: f32,
    pub max: f32,
    pub buffer_size: usize,
}

impl MonitorGraphControl {
    pub fn new(min: f32, max: f32, buffer_size: usize) -> Self {
        Self {
            values: VecDeque::with_capacity(buffer_size),
            min,
            max,
            buffer_size,
        }
    }

    /// Push a new value, evicting the oldest if at capacity.
    pub fn push(&mut self, value: f32) {
        if self.values.len() >= self.buffer_size {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    /// Normalized height [0, 1] for a given value.
    fn normalized(&self, value: f32) -> f32 {
        let range = self.max - self.min;
        if range.abs() < f32::EPSILON {
            return 0.5;
        }
        ((value - self.min) / range).clamp(0.0, 1.0)
    }

    /// Current (latest) value formatted.
    fn current_text(&self) -> String {
        match self.values.back() {
            Some(v) => format!("{v:.1}"),
            None => "—".to_string(),
        }
    }
}

/// Marker on the graph bars container.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct MonitorGraphBars;

/// Marker on the graph current value text.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct MonitorGraphValueText;

/// Marker on individual graph bar entities.
#[derive(Component, Clone, Debug)]
pub(crate) struct MonitorGraphBar(pub usize);

/// Spawn a graph monitor.
pub(crate) fn spawn_monitor_graph_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    control: &MonitorGraphControl,
    asset_server: &AssetServer,
) -> Entity {
    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row-graph"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            control.clone(),
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            // Header: label + current value
            row.spawn((Node::default(), ClassList::new("pane-graph-header")))
                .with_children(|header| {
                    header.spawn((
                        Text::new(&meta.label),
                        label_font(),
                        ClassList::new("pane-label-text"),
                    ));
                    header.spawn((
                        Text::new(control.current_text()),
                        pane_font(10.0),
                        ClassList::new("pane-graph-value-text"),
                        MonitorGraphValueText,
                    ));
                });

            // Graph area with pre-allocated bars
            row.spawn((
                Node::default(),
                ClassList::new("pane-graph-area"),
                MonitorGraphBars,
            ))
            .with_children(|area| {
                for i in 0..control.buffer_size {
                    let height_pct = if i < control.values.len() {
                        control.normalized(control.values[i]) * 100.0
                    } else {
                        0.0
                    };
                    area.spawn((
                        Node {
                            flex_grow: 1.0,
                            height: Val::Percent(height_pct),
                            ..default()
                        },
                        ClassList::new("pane-graph-bar"),
                        MonitorGraphBar(i),
                    ));
                }
            });
        });

    row_entity
}

/// System: update graph bars and value text when MonitorGraphControl changes.
pub(crate) fn update_monitor_graph(
    q_graphs: Query<(Entity, &MonitorGraphControl), Changed<MonitorGraphControl>>,
    q_children: Query<&Children>,
    mut q_bars: Query<(&MonitorGraphBar, &mut Node)>,
    mut q_text: Query<&mut Text, With<MonitorGraphValueText>>,
) {
    for (entity, graph) in &q_graphs {
        // Update value text
        let current = graph.current_text();
        for descendant in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_text.get_mut(descendant) {
                text.0 = current.clone();
            }
        }

        // Update bar heights
        let num_values = graph.values.len();
        for descendant in q_children.iter_descendants(entity) {
            if let Ok((bar, mut node)) = q_bars.get_mut(descendant) {
                let idx = bar.0;
                // Shift bars: bar index maps to value index relative to the end
                // Bar 0 = oldest visible, bar N-1 = newest
                let value_idx = if num_values >= graph.buffer_size {
                    idx
                } else if idx >= graph.buffer_size - num_values {
                    idx - (graph.buffer_size - num_values)
                } else {
                    // No value for this bar yet
                    node.height = Val::Percent(0.0);
                    continue;
                };

                if value_idx < num_values {
                    let h = graph.normalized(graph.values[value_idx]) * 100.0;
                    node.height = Val::Percent(h);
                } else {
                    node.height = Val::Percent(0.0);
                }
            }
        }
    }
}
