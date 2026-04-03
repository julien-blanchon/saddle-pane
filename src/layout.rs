use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;
use bevy_input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy_ui_widgets::{Activate, Button, ControlOrientation, CoreScrollbarThumb, Scrollbar, observe};

use std::collections::HashMap;

use crate::builder::{ControlSpec, LayoutItem, PaneSpec, TabSpec};
use crate::registry::{PaneControlRegistry, SpawnFn, DefaultValueFn};
use crate::controls::InitialValue;
use crate::controls::PaneControlMeta;
use crate::controls::PaneValue;
use crate::controls::button::spawn_button_ui;
use crate::controls::color::{ColorControl, spawn_color_ui};
use crate::controls::monitor::{
    MonitorControl, MonitorGraphControl, MonitorLogControl, spawn_monitor_graph_ui,
    spawn_monitor_log_ui, spawn_monitor_ui,
};
use crate::controls::number::{NumberControl, spawn_number_ui};
use crate::controls::select::{SelectControl, spawn_select_ui};
use crate::controls::separator::spawn_separator_ui;
use crate::controls::slider::{SliderControl, spawn_slider_ui};
use crate::controls::text::{TextControl, spawn_text_ui};
use crate::controls::toggle::{ToggleControl, spawn_toggle_ui};
use crate::controls::{label_font, pane_font};
use crate::icons::collect_icon_svgs;
use crate::store::PaneStore;
use crate::style;

/// Marker for the root entity of a pane.
#[derive(Component, Clone, Debug)]
pub struct PaneRoot {
    pub title: String,
    pub collapsed: bool,
}

/// Marker for pane title bar.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneTitleBar;

/// Marker for the drag handle inside the title bar.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneDragHandle;

/// Marker for pane body.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneBody;

/// Marker for collapsible folder.
#[derive(Component, Clone, Debug)]
pub struct PaneFolder {
    pub label: String,
    pub collapsed: bool,
}

/// Marker for folder body (the part that hides/shows).
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneFolderBody;

/// Marker for folder collapse icon (CSS-based arrow).
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneFolderIcon;

/// Tracks pane drag state (start position of the pane when drag began).
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneDragState {
    dragging: bool,
    start_left: f32,
    start_top: f32,
}

/// Marker for the resize handle on the right edge of the pane.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneResizeHandle;

/// Holds the current search/filter text for a pane.
#[derive(Component, Clone, Debug, Default)]
pub struct PaneSearchFilter(pub String);

/// Marker for the search input text entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneSearchText;

/// Tracks pane resize state.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneResizeState {
    resizing: bool,
    start_width: f32,
}

// ── Tab components ──

/// Marker for the tab group root entity.
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneTabGroup;

/// Component on tab button — stores which page index it controls.
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneTabButton {
    pub index: usize,
}

/// Component on tab page — stores its index.
#[derive(Component, Clone, Debug)]
pub(crate) struct PaneTabPage {
    pub index: usize,
}

const TAB_CSS: &str = "embedded://saddle_pane/style/tab.css";

/// Snapshot of custom control plugins, extracted from the registry for spawning.
struct CustomPlugins {
    spawn_fns: HashMap<String, SpawnFn>,
    default_fns: HashMap<String, DefaultValueFn>,
}

/// Pre-rasterized SVG icon handles (SVG string → Image handle).
type IconCache = HashMap<String, Handle<Image>>;

/// Spawn a complete pane from a builder spec.
pub fn spawn_pane(commands: &mut Commands, spec: PaneSpec) -> Entity {
    // We need the AssetServer — schedule a deferred command
    let entity = commands.spawn_empty().id();
    commands.queue(SpawnPaneCommand { entity, spec });
    entity
}

struct SpawnPaneCommand {
    entity: Entity,
    spec: PaneSpec,
}

impl Command for SpawnPaneCommand {
    fn apply(self, world: &mut World) {
        let asset_server = world.resource::<AssetServer>().clone();

        // Snapshot custom plugin functions from the registry
        let custom_plugins = {
            let registry = world.resource::<PaneControlRegistry>();
            CustomPlugins {
                spawn_fns: registry.spawn_fns(),
                default_fns: registry.default_fns(),
            }
        };

        // Pre-rasterize all icon SVGs to Image handles (world borrow is scoped)
        let (icon_cache, grip_icon_handle): (IconCache, Option<Handle<Image>>) = {
            use crate::icons::{svg_to_image, ICON_GRIP_VERTICAL};
            let mut images = world.resource_mut::<Assets<Image>>();
            let mut cache = HashMap::new();
            collect_icon_svgs(&self.spec.items, &mut |svg: &str| {
                if !cache.contains_key(svg) {
                    if let Some(image) = svg_to_image(svg, 48) {
                        cache.insert(svg.to_string(), images.add(image));
                    }
                }
            });
            collect_icon_svgs(&self.spec.footer, &mut |svg: &str| {
                if !cache.contains_key(svg) {
                    if let Some(image) = svg_to_image(svg, 48) {
                        cache.insert(svg.to_string(), images.add(image));
                    }
                }
            });
            let grip = svg_to_image(ICON_GRIP_VERTICAL, 48).map(|img| images.add(img));
            (cache, grip)
        };

        // Build the pane entity hierarchy
        let spec_width = self.spec.width;
        let searchable = self.spec.searchable;
        let collapsed = self.spec.collapsed;
        let title = self.spec.title.clone();

        // Only set position_type + position + width in Rust (dynamic); everything else via CSS
        let mut node = Node {
            position_type: PositionType::Absolute,
            ..default()
        };

        if let Some(w) = spec_width {
            node.width = Val::Px(w);
        }

        use crate::builder::PanePosition;
        let margin = 8.0;
        match self.spec.position {
            Some(PanePosition::Absolute(x, y)) => {
                node.left = Val::Px(x);
                node.top = Val::Px(y);
            }
            Some(PanePosition::TopLeft) => {
                node.left = Val::Px(margin);
                node.top = Val::Px(margin);
            }
            Some(PanePosition::TopRight) | None => {
                node.right = Val::Px(margin);
                node.top = Val::Px(margin);
            }
            Some(PanePosition::BottomLeft) => {
                node.left = Val::Px(margin);
                node.bottom = Val::Px(margin);
            }
            Some(PanePosition::BottomRight) => {
                node.right = Val::Px(margin);
                node.bottom = Val::Px(margin);
            }
        }

        world.entity_mut(self.entity).insert((
            node,
            PaneRoot {
                title: title.clone(),
                collapsed,
            },
            PaneDragState::default(),
            PaneResizeState::default(),
            if collapsed {
                ClassList::new("pane is-collapsed")
            } else {
                ClassList::new("pane")
            },
            NodeStyleSheet::new(asset_server.load(style::PANE_CSS)),
            InlineStyle::default(),
            GlobalZIndex(900),
        ));

        if searchable {
            world
                .entity_mut(self.entity)
                .insert(PaneSearchFilter::default());
        }

        // Spawn children using commands
        let mut commands = world.commands();
        commands.entity(self.entity).with_children(|pane| {
            // Header row: drag handle | divider | title bar (collapse)
            pane.spawn((
                Node::default(),
                ClassList::new("pane-header"),
            ))
            .with_children(|header| {
                // Drag handle — separate from title bar so clicks don't trigger collapse
                {
                    let mut drag_handle = header.spawn((
                        Node {
                            width: Val::Px(16.0),
                            height: Val::Px(24.0),
                            min_width: Val::Px(16.0),
                            min_height: Val::Px(24.0),
                            flex_shrink: 0.0,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        PaneDragHandle,
                        ClassList::new("pane-drag-handle"),
                        observe(on_drag_handle_drag_start),
                        observe(on_drag_handle_drag),
                        observe(on_drag_handle_drag_end),
                    ));
                    if let Some(ref handle) = grip_icon_handle {
                        drag_handle.with_children(|dh| {
                            dh.spawn((
                                Node {
                                    width: Val::Px(10.0),
                                    height: Val::Px(10.0),
                                    min_width: Val::Px(10.0),
                                    min_height: Val::Px(10.0),
                                    flex_shrink: 0.0,
                                    ..default()
                                },
                                ImageNode::new(handle.clone()),
                            ));
                        });
                    }
                }

                // Vertical divider between drag handle and title
                header.spawn((
                    Node::default(),
                    ClassList::new("pane-header-divider"),
                ));

                // Title bar — collapse on click
                header.spawn((
                    Node::default(),
                    Interaction::default(),
                    Button,
                    PaneTitleBar,
                    ClassList::new("pane-title"),
                    observe(on_title_activate),
                ))
                .with_children(|title_bar| {
                    // CSS-based chevron arrow (border trick, rotates via .is-collapsed)
                    title_bar.spawn((Node::default(), ClassList::new("pane-collapse-icon")));
                    title_bar.spawn((
                        Text::new(&title),
                        label_font(),
                        ClassList::new("pane-title-text"),
                    ));
                });
            });

            // Search bar (optional)
            if searchable {
                pane.spawn((
                    Node {
                        display: Display::Flex,
                        height: Val::Px(24.0),
                        min_height: Val::Px(24.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                        ..default()
                    },
                    ClassList::new("pane-search-bar"),
                ))
                .with_children(|search_bar| {
                    search_bar.spawn((
                        Text::new(""),
                        pane_font(10.0),
                        ClassList::new("pane-search-input"),
                        PaneSearchText,
                    ));
                });
            }

            // Body — scroll viewport + scrollbar in a flex row
            let mut body_entity = Entity::PLACEHOLDER;
            pane.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    min_height: Val::ZERO,
                    max_height: Val::Px(480.0),
                    ..default()
                },
                ClassList::new("pane-body-frame"),
            ))
            .with_children(|frame| {
                // Scroll viewport
                frame
                    .spawn((
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            min_width: Val::ZERO,
                            min_height: Val::ZERO,
                            padding: UiRect::axes(Val::ZERO, Val::Px(2.0)),
                            overflow: Overflow {
                                x: OverflowAxis::Clip,
                                y: OverflowAxis::Scroll,
                            },
                            ..default()
                        },
                        ScrollPosition::default(),
                        PaneBody,
                        TabGroup::new(0),
                        ClassList::new("pane-body"),
                    ))
                    .with_children(|body| {
                        body_entity = body.target_entity();
                        for item in &self.spec.items {
                            spawn_layout_item(body, item, &title, &asset_server, &custom_plugins, &icon_cache);
                        }
                    });

                // Scrollbar — Scrollbar component wired to body_entity
                frame
                    .spawn((
                        Node {
                            width: Val::Px(4.0),
                            min_width: Val::Px(4.0),
                            ..default()
                        },
                        Scrollbar::new(
                            body_entity,
                            ControlOrientation::Vertical,
                            16.0,
                        ),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Px(4.0),
                                min_height: Val::Px(16.0),
                                border_radius: BorderRadius::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.24, 0.24, 0.27, 1.0)),
                            CoreScrollbarThumb,
                        ));
                    });
            });

            // Footer — pinned at bottom, outside scroll viewport
            if !self.spec.footer.is_empty() {
                pane.spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        min_height: Val::Px(28.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        column_gap: Val::Px(4.0),
                        border: UiRect::top(Val::Px(1.0)),
                        ..default()
                    },
                    ClassList::new("pane-footer"),
                ))
                .with_children(|footer| {
                    for item in &self.spec.footer {
                        spawn_layout_item(footer, item, &title, &asset_server, &custom_plugins, &icon_cache);
                    }
                });
            }

            // Resize handle — thin strip on right edge
            pane.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Px(4.0),
                    ..default()
                },
                PaneResizeHandle,
                ClassList::new("pane-resize-handle"),
                observe(on_resize_drag_start),
                observe(on_resize_drag),
                observe(on_resize_drag_end),
            ));
        });

        // Initialize store values
        let mut store = world.resource_mut::<PaneStore>();
        init_store_values(&mut store, &title, &self.spec.items, &custom_plugins);
        init_store_values(&mut store, &title, &self.spec.footer, &custom_plugins);
    }
}

fn spawn_layout_item(
    parent: &mut ChildSpawnerCommands,
    item: &LayoutItem,
    pane_title: &str,
    asset_server: &AssetServer,
    custom: &CustomPlugins,
    icon_cache: &IconCache,
) {
    match item {
        LayoutItem::Control(spec) => {
            spawn_control(parent, spec, pane_title, asset_server, custom, icon_cache);
        }
        LayoutItem::TabGroup { tabs, active } => {
            spawn_tab_group(parent, tabs, *active, pane_title, asset_server, custom, icon_cache);
        }
        LayoutItem::Folder {
            label,
            items,
            collapsed,
        } => {
            let is_collapsed = *collapsed;
            let body_class = if is_collapsed {
                "pane-folder-body"
            } else {
                "pane-folder-body is-open"
            };

            parent
                .spawn((
                    Node::default(),
                    ClassList::new("pane-folder"),
                    NodeStyleSheet::new(asset_server.load(style::FOLDER_CSS)),
                    PaneFolder {
                        label: label.clone(),
                        collapsed: is_collapsed,
                    },
                ))
                .with_children(|folder| {
                    // Folder header
                    folder
                        .spawn((
                            Node::default(),
                            Interaction::default(),
                            Button,
                            ClassList::new("pane-folder-header"),
                            observe(on_folder_activate),
                        ))
                        .with_children(|header| {
                            // CSS-based chevron arrow (toggles is-open class)
                            let icon_class = if is_collapsed {
                                "pane-folder-icon"
                            } else {
                                "pane-folder-icon is-open"
                            };
                            header.spawn((
                                Node::default(),
                                PaneFolderIcon,
                                ClassList::new(icon_class),
                            ));
                            header.spawn((
                                Text::new(label),
                                pane_font(10.0),
                                ClassList::new("pane-folder-title"),
                            ));
                        });

                    // Folder body
                    folder
                        .spawn((Node::default(), ClassList::new(body_class), PaneFolderBody))
                        .with_children(|body| {
                            for child_item in items {
                                spawn_layout_item(body, child_item, pane_title, asset_server, custom, icon_cache);
                            }
                        });
                });
        }
    }
}

fn spawn_control(
    parent: &mut ChildSpawnerCommands,
    spec: &ControlSpec,
    pane_title: &str,
    asset_server: &AssetServer,
    custom: &CustomPlugins,
    icon_cache: &IconCache,
) {
    match spec {
        ControlSpec::Slider {
            label,
            min,
            max,
            step,
            default,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let control = SliderControl {
                value: *default,
                min: *min,
                max: *max,
                step: *step,
            };
            let row = spawn_slider_ui(parent, &meta, &control, asset_server);
            let mut cmds = parent.commands();
            cmds.entity(row)
                .insert(InitialValue(PaneValue::Float(*default)));
            apply_row_extras(&mut cmds, row, &meta);
        }
        ControlSpec::Toggle {
            label,
            default,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let control = ToggleControl { value: *default };
            let row = spawn_toggle_ui(parent, &meta, &control, asset_server);
            let mut cmds = parent.commands();
            cmds.entity(row)
                .insert(InitialValue(PaneValue::Bool(*default)));
            apply_row_extras(&mut cmds, row, &meta);
        }
        ControlSpec::Button {
            label,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let row = spawn_button_ui(parent, &meta, asset_server);
            apply_row_extras(&mut parent.commands(), row, &meta);
        }
        ControlSpec::Number {
            label,
            default,
            min,
            max,
            step,
            step_buttons,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let control = NumberControl {
                value: *default,
                min: *min,
                max: *max,
                step: *step,
                show_step_buttons: *step_buttons,
            };
            let row = spawn_number_ui(parent, &meta, &control, asset_server);
            let mut cmds = parent.commands();
            cmds.entity(row)
                .insert(InitialValue(PaneValue::Float(*default)));
            apply_row_extras(&mut cmds, row, &meta);
        }
        ControlSpec::Text {
            label,
            default,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let control = TextControl {
                value: default.clone(),
            };
            let row = spawn_text_ui(parent, &meta, &control, asset_server);
            let mut cmds = parent.commands();
            cmds.entity(row)
                .insert(InitialValue(PaneValue::String(default.clone())));
            apply_row_extras(&mut cmds, row, &meta);
        }
        ControlSpec::Select {
            label,
            options,
            default,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let control = SelectControl {
                value: *default,
                options: options.clone(),
            };
            let row = spawn_select_ui(parent, &meta, &control, asset_server);
            let mut cmds = parent.commands();
            cmds.entity(row)
                .insert(InitialValue(PaneValue::Int(*default as i64)));
            apply_row_extras(&mut cmds, row, &meta);
        }
        ControlSpec::Color {
            label,
            default,
            tooltip,
            icon,
        } => {
            let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
            let control = ColorControl { value: *default };
            let row = spawn_color_ui(parent, &meta, &control, asset_server);
            let mut cmds = parent.commands();
            cmds.entity(row)
                .insert(InitialValue(PaneValue::Color(*default)));
            apply_row_extras(&mut cmds, row, &meta);
        }
        ControlSpec::Monitor { label, default } => {
            let meta = make_meta(pane_title, label, None, None, icon_cache);
            let control = MonitorControl {
                value: default.clone(),
            };
            spawn_monitor_ui(parent, &meta, &control, asset_server);
        }
        ControlSpec::MonitorLog {
            label,
            buffer_size,
        } => {
            let meta = make_meta(pane_title, label, None, None, icon_cache);
            let control = MonitorLogControl::new(*buffer_size);
            spawn_monitor_log_ui(parent, &meta, &control, asset_server);
        }
        ControlSpec::MonitorGraph {
            label,
            min,
            max,
            buffer_size,
        } => {
            let meta = make_meta(pane_title, label, None, None, icon_cache);
            let control = MonitorGraphControl::new(*min as f32, *max as f32, *buffer_size);
            spawn_monitor_graph_ui(parent, &meta, &control, asset_server);
        }
        ControlSpec::Custom {
            control_id,
            label,
            config,
            tooltip,
            icon,
        } => {
            if let Some(spawn_fn) = custom.spawn_fns.get(control_id) {
                let meta = make_meta(pane_title, label, tooltip.as_deref(), icon.as_deref(), icon_cache);
                let row = spawn_fn(parent, &meta, config, asset_server);
                let mut cmds = parent.commands();
                // Set initial value from plugin's default_value function
                if let Some(default_fn) = custom.default_fns.get(control_id) {
                    if let Some(value) = default_fn(config) {
                        cmds.entity(row).insert(InitialValue(value));
                    }
                }
                apply_row_extras(&mut cmds, row, &meta);
            } else {
                warn!("Unknown custom control '{control_id}' — no plugin registered");
            }
        }
        ControlSpec::Separator => {
            spawn_separator_ui(parent, asset_server);
        }
    }
}

fn make_meta(
    pane_title: &str,
    label: &str,
    tooltip: Option<&str>,
    icon: Option<&str>,
    icon_cache: &IconCache,
) -> PaneControlMeta {
    let icon_handle = icon
        .and_then(|s| icon_cache.get(s))
        .cloned();
    PaneControlMeta {
        pane_title: pane_title.to_string(),
        label: label.to_string(),
        tooltip: tooltip.map(|s| s.to_string()),
        order: 0,
        icon: icon.map(|s| s.to_string()),
        icon_handle,
    }
}

/// After spawning a control row, apply extra components (Interaction for tooltip hover).
fn apply_row_extras(commands: &mut Commands, row: Entity, meta: &PaneControlMeta) {
    // Add Interaction so hover detection works for tooltips
    if meta.tooltip.is_some() {
        commands.entity(row).insert(Interaction::default());
    }
}

fn init_store_values(store: &mut PaneStore, pane_title: &str, items: &[LayoutItem], custom: &CustomPlugins) {
    for item in items {
        match item {
            LayoutItem::Control(spec) => {
                if let Some((label, value)) = spec_to_value(spec, custom) {
                    store.init(pane_title, &label, value);
                }
            }
            LayoutItem::Folder { items, .. } => {
                init_store_values(store, pane_title, items, custom);
            }
            LayoutItem::TabGroup { tabs, .. } => {
                for tab in tabs {
                    init_store_values(store, pane_title, &tab.items, custom);
                }
            }
        }
    }
}

fn spec_to_value(spec: &ControlSpec, custom: &CustomPlugins) -> Option<(String, PaneValue)> {
    match spec {
        ControlSpec::Slider { label, default, .. } => {
            Some((label.clone(), PaneValue::Float(*default)))
        }
        ControlSpec::Toggle { label, default, .. } => {
            Some((label.clone(), PaneValue::Bool(*default)))
        }
        ControlSpec::Number { label, default, .. } => {
            Some((label.clone(), PaneValue::Float(*default)))
        }
        ControlSpec::Text { label, default, .. } => {
            Some((label.clone(), PaneValue::String(default.clone())))
        }
        ControlSpec::Select { label, default, .. } => {
            Some((label.clone(), PaneValue::Int(*default as i64)))
        }
        ControlSpec::Color { label, default, .. } => {
            Some((label.clone(), PaneValue::Color(*default)))
        }
        ControlSpec::Monitor { label, default } => {
            Some((label.clone(), PaneValue::String(default.clone())))
        }
        ControlSpec::Custom {
            control_id,
            label,
            config,
            ..
        } => {
            if let Some(default_fn) = custom.default_fns.get(control_id) {
                default_fn(config).map(|v| (label.clone(), v))
            } else {
                None
            }
        }
        ControlSpec::Button { .. }
        | ControlSpec::Separator
        | ControlSpec::MonitorLog { .. }
        | ControlSpec::MonitorGraph { .. } => None,
    }
}

// ── Tab group spawning ──

fn spawn_tab_group(
    parent: &mut ChildSpawnerCommands,
    tabs: &[TabSpec],
    active: usize,
    pane_title: &str,
    asset_server: &AssetServer,
    custom: &CustomPlugins,
    icon_cache: &IconCache,
) {
    parent
        .spawn((
            Node::default(),
            PaneTabGroup,
            ClassList::new("pane-tab-group"),
            NodeStyleSheet::new(asset_server.load(TAB_CSS)),
        ))
        .with_children(|group| {
            // Tab bar with buttons
            group
                .spawn((
                    Node {
                        overflow: Overflow {
                            x: OverflowAxis::Scroll,
                            y: OverflowAxis::Clip,
                        },
                        ..default()
                    },
                    ScrollPosition::default(),
                    TabGroup::new(0),
                    ClassList::new("pane-tab-bar"),
                ))
                .with_children(|bar| {
                    for (i, tab) in tabs.iter().enumerate() {
                        let is_active = i == active;
                        let btn_class = if is_active {
                            "pane-tab-button is-active"
                        } else {
                            "pane-tab-button"
                        };
                        bar.spawn((
                            Node::default(),
                            Interaction::default(),
                            Button,
                            PaneTabButton { index: i },
                            ClassList::new(btn_class),
                            AutoDirectionalNavigation::default(),
                            TabIndex(0),
                            observe(on_tab_activate),
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(&tab.label),
                                pane_font(10.0),
                                ClassList::new("pane-tab-button-text"),
                            ));
                        });
                    }
                });

            // Tab content pages
            group
                .spawn((Node::default(), ClassList::new("pane-tab-content")))
                .with_children(|content| {
                    for (i, tab) in tabs.iter().enumerate() {
                        let is_active = i == active;
                        let page_class = if is_active {
                            "pane-tab-page is-active"
                        } else {
                            "pane-tab-page"
                        };
                        content
                            .spawn((
                                Node::default(),
                                PaneTabPage { index: i },
                                ClassList::new(page_class),
                            ))
                            .with_children(|page| {
                                for item in &tab.items {
                                    spawn_layout_item(page, item, pane_title, asset_server, custom, icon_cache);
                                }
                            });
                    }
                });
        });
}

fn on_tab_activate(
    ev: On<Activate>,
    q_tab_btn: Query<&PaneTabButton>,
    q_parent: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut q_btn_classes: Query<(&PaneTabButton, &mut ClassList), Without<PaneTabPage>>,
    mut q_page_classes: Query<(&PaneTabPage, &mut ClassList), Without<PaneTabButton>>,
) {
    let Ok(tab_btn) = q_tab_btn.get(ev.entity) else {
        return;
    };
    let active_index = tab_btn.index;

    // Walk up: tab_button → tab_bar → tab_group
    let Ok(bar_of) = q_parent.get(ev.entity) else {
        return;
    };
    let Ok(group_of) = q_parent.get(bar_of.parent()) else {
        return;
    };
    let group_entity = group_of.parent();

    // Collect descendants to avoid borrow issues
    let descendants: Vec<Entity> = q_children.iter_descendants(group_entity).collect();

    // Update tab button active states
    for entity in &descendants {
        if let Ok((btn, mut classes)) = q_btn_classes.get_mut(*entity) {
            if btn.index == active_index {
                classes.add("is-active");
            } else {
                classes.remove("is-active");
            }
        }
    }

    // Update tab page visibility
    for entity in &descendants {
        if let Ok((page, mut classes)) = q_page_classes.get_mut(*entity) {
            if page.index == active_index {
                classes.add("is-active");
            } else {
                classes.remove("is-active");
            }
        }
    }
}

// ── Interaction handlers ──

fn on_title_activate(
    ev: On<Activate>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<(&mut PaneRoot, &mut ClassList)>,
) {
    // Walk: title_bar → header → PaneRoot
    let Ok(child_of) = q_parent.get(ev.entity) else {
        return;
    };
    let header_entity = child_of.parent();
    let Ok(header_of) = q_parent.get(header_entity) else {
        return;
    };
    let pane_entity = header_of.parent();
    let Ok((mut pane_root, mut classes)) = q_pane.get_mut(pane_entity) else {
        return;
    };

    pane_root.collapsed = !pane_root.collapsed;
    if pane_root.collapsed {
        classes.add("is-collapsed");
    } else {
        classes.remove("is-collapsed");
    }
}

fn on_folder_activate(
    ev: On<Activate>,
    q_parent: Query<&ChildOf>,
    mut q_folder: Query<&mut PaneFolder>,
    q_children: Query<&Children>,
    mut q_body: Query<&mut ClassList, (With<PaneFolderBody>, Without<PaneFolderIcon>)>,
    mut q_icon: Query<&mut ClassList, (With<PaneFolderIcon>, Without<PaneFolderBody>)>,
) {
    // Header is child of folder
    let Ok(child_of) = q_parent.get(ev.entity) else {
        return;
    };
    let folder_entity = child_of.parent();
    let Ok(mut folder) = q_folder.get_mut(folder_entity) else {
        return;
    };

    folder.collapsed = !folder.collapsed;

    // Update folder body class
    for child in q_children.iter_descendants(folder_entity) {
        if let Ok(mut classes) = q_body.get_mut(child) {
            if folder.collapsed {
                classes.remove("is-open");
            } else {
                classes.add("is-open");
            }
        }
    }

    // Update folder icon class (CSS rotation)
    for child in q_children.iter_descendants(folder_entity) {
        if let Ok(mut classes) = q_icon.get_mut(child) {
            if folder.collapsed {
                classes.remove("is-open");
            } else {
                classes.add("is-open");
            }
            break;
        }
    }
}

// ── Drag handlers ──
//
// The drag handle lives inside the title bar: PaneRoot → PaneTitleBar → PaneDragHandle.
// We walk up two parents to reach the PaneRoot.

/// Walk from the drag handle entity up to the PaneRoot entity.
fn drag_handle_to_pane(entity: Entity, q_parent: &Query<&ChildOf>) -> Option<Entity> {
    // handle → header
    let header = q_parent.get(entity).ok()?.parent();
    // header → pane_root
    let pane_root = q_parent.get(header).ok()?.parent();
    Some(pane_root)
}

fn on_drag_handle_drag_start(
    mut ev: On<Pointer<DragStart>>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<
        (&mut Node, &UiGlobalTransform, &ComputedNode, &mut PaneDragState),
        With<PaneRoot>,
    >,
) {
    // Stop propagation so the title bar's Activate handler doesn't fire (collapse).
    ev.propagate(false);

    let Some(pane_entity) = drag_handle_to_pane(ev.entity, &q_parent) else {
        return;
    };
    let Ok((mut node, global_transform, computed, mut drag)) = q_pane.get_mut(pane_entity) else {
        return;
    };

    drag.dragging = true;

    // If `left`/`top` are not Px (e.g. the pane was positioned with `right`),
    // resolve the actual pixel position from the computed layout and switch to left/top
    // so subsequent delta-based dragging works correctly.
    //
    // UiGlobalTransform's translation points to the CENTER of the node in physical pixels.
    // ComputedNode::size is also in physical pixels. We convert to logical pixels using
    // inverse_scale_factor before assigning to node.left/top.
    let center = global_transform.affine().translation;
    let half_size_physical = computed.size() / 2.0;
    let inv_scale = computed.inverse_scale_factor;

    let left = match node.left {
        Val::Px(v) => v,
        _ => {
            let resolved = (center.x - half_size_physical.x) * inv_scale;
            node.left = Val::Px(resolved);
            node.right = Val::Auto;
            resolved
        }
    };
    let top = match node.top {
        Val::Px(v) => v,
        _ => {
            let resolved = (center.y - half_size_physical.y) * inv_scale;
            node.top = Val::Px(resolved);
            node.bottom = Val::Auto;
            resolved
        }
    };

    drag.start_left = left;
    drag.start_top = top;
}

fn on_drag_handle_drag(
    mut ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<(&mut Node, &PaneDragState), With<PaneRoot>>,
) {
    ev.propagate(false);

    let Some(pane_entity) = drag_handle_to_pane(ev.entity, &q_parent) else {
        return;
    };
    let Ok((mut node, drag)) = q_pane.get_mut(pane_entity) else {
        return;
    };
    if !drag.dragging {
        return;
    }

    let delta = ev.event().delta;
    let current_left = match node.left {
        Val::Px(v) => v,
        _ => drag.start_left,
    };
    let current_top = match node.top {
        Val::Px(v) => v,
        _ => drag.start_top,
    };

    node.left = Val::Px(current_left + delta.x);
    node.top = Val::Px(current_top + delta.y);
    node.right = Val::Auto;
    node.bottom = Val::Auto;
}

fn on_drag_handle_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<&mut PaneDragState, With<PaneRoot>>,
) {
    ev.propagate(false);

    let Some(pane_entity) = drag_handle_to_pane(ev.entity, &q_parent) else {
        return;
    };
    if let Ok(mut drag) = q_pane.get_mut(pane_entity) {
        drag.dragging = false;
    }
}

// ── Resize handlers ──
//
// The resize handle is a direct child of PaneRoot.

const PANE_MIN_WIDTH: f32 = 200.0;
const PANE_MAX_WIDTH: f32 = 600.0;

fn on_resize_drag_start(
    mut ev: On<Pointer<DragStart>>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<(&Node, &ComputedNode, &mut PaneResizeState), With<PaneRoot>>,
) {
    ev.propagate(false);

    let Ok(child_of) = q_parent.get(ev.entity) else { return };
    let Ok((node, computed, mut resize)) = q_pane.get_mut(child_of.parent()) else { return };

    resize.resizing = true;
    // Get current width: prefer Node.width if Px, otherwise use computed size
    resize.start_width = match node.width {
        Val::Px(v) => v,
        _ => computed.size().x * computed.inverse_scale_factor,
    };
}

fn on_resize_drag(
    mut ev: On<Pointer<Drag>>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<(&mut Node, &PaneResizeState), With<PaneRoot>>,
) {
    ev.propagate(false);

    let Ok(child_of) = q_parent.get(ev.entity) else { return };
    let Ok((mut node, resize)) = q_pane.get_mut(child_of.parent()) else { return };
    if !resize.resizing { return; }

    let current_width = match node.width {
        Val::Px(v) => v,
        _ => resize.start_width,
    };
    let new_width = (current_width + ev.event().delta.x).clamp(PANE_MIN_WIDTH, PANE_MAX_WIDTH);
    node.width = Val::Px(new_width);
}

fn on_resize_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    q_parent: Query<&ChildOf>,
    mut q_pane: Query<&mut PaneResizeState, With<PaneRoot>>,
) {
    ev.propagate(false);

    let Ok(child_of) = q_parent.get(ev.entity) else { return };
    if let Ok(mut resize) = q_pane.get_mut(child_of.parent()) {
        resize.resizing = false;
    }
}
