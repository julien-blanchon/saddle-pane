//! # saddle_pane_file
//!
//! File loader plugin for [saddle_pane](https://github.com/your/saddle_pane).
//!
//! Adds a "Browse" button + path display. When clicked, opens a native file
//! dialog. The selected path is stored as a `PaneValue::String`.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;

use saddle_pane::controls::{PaneControlMeta, PaneValue, pane_font, spawn_label};
use saddle_pane::events::PaneChanged;
use saddle_pane::prelude::{PaneControlPlugin, PaneControlRegistry, PaneSystems};
use saddle_pane::registry::ControlConfig;
use saddle_pane::store::PaneStore;

const STYLE_PATH: &str = "embedded://saddle_pane_file/style/file.css";

// ══════════════════════════════════════════════════════════════════════
// Public types
// ══════════════════════════════════════════════════════════════════════

/// Component storing the file control state.
#[derive(Component, Clone, Debug)]
pub struct FileControl {
    pub path: Option<String>,
    pub extensions: Vec<String>,
    pub dialog_title: String,
    /// Internal: triggers the file dialog next frame.
    pub(crate) pending_open: bool,
    /// True while an async dialog is in flight for this control.
    pub(crate) dialog_in_flight: bool,
}

/// Resource holding the channel for file dialog results.
#[derive(Resource)]
struct FileDialogChannel {
    tx: std::sync::mpsc::Sender<(Entity, Option<String>)>,
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<(Entity, Option<String>)>>,
}

impl Default for FileDialogChannel {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx: std::sync::Mutex::new(rx),
        }
    }
}

#[derive(Component, Clone, Debug, Default)]
struct FilePathText;

#[derive(Component, Clone, Debug, Default)]
struct FilePathArea;

// ══════════════════════════════════════════════════════════════════════
// Plugin
// ══════════════════════════════════════════════════════════════════════

pub struct PaneFilePlugin;

impl Plugin for PaneFilePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "style/file.css");

        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "file",
            build: build_systems,
            spawn: spawn_file_ui,
            default_value: |_| Some(PaneValue::String(String::new())),
        });

        build_systems(app);
    }
}

fn build_systems(app: &mut App) {
    app.init_resource::<FileDialogChannel>();
    app.add_systems(
        PostUpdate,
        (launch_file_dialogs, collect_file_results, update_file_display, sync_file_to_store)
            .chain()
            .in_set(PaneSystems::Display),
    );
}

// ══════════════════════════════════════════════════════════════════════
// Spawn
// ══════════════════════════════════════════════════════════════════════

fn spawn_file_ui(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let extensions: Vec<String> = config
        .get_string_list("extensions")
        .map(|s| s.to_vec())
        .unwrap_or_default();
    let dialog_title = config
        .get_string("dialog_title")
        .unwrap_or("Select File")
        .to_string();

    let mut row_entity = Entity::PLACEHOLDER;

    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-row"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            meta.clone(),
            FileControl {
                path: None,
                extensions,
                dialog_title,
                pending_open: false,
                dialog_in_flight: false,
            },
        ))
        .with_children(|row| {
            row_entity = row.target_entity();

            spawn_label(row, &meta.label);

            row.spawn((Node::default(), ClassList::new("pane-file-area")))
                .with_children(|area| {
                    // Browse button
                    area.spawn((
                        Node::default(),
                        Interaction::default(),
                        bevy_ui_widgets::Button,
                        ClassList::new("pane-file-browse-btn"),
                        bevy_ui_widgets::observe(on_browse_click),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("Browse"),
                            pane_font(10.0),
                            ClassList::new("pane-file-browse-text"),
                        ));
                    });

                    // Path display
                    area.spawn((
                        Node::default(),
                        ClassList::new("pane-file-path-area"),
                        FilePathArea,
                    ))
                    .with_children(|path_area| {
                        path_area.spawn((
                            Text::new("No file selected"),
                            pane_font(10.0),
                            ClassList::new("pane-file-path-text"),
                            FilePathText,
                        ));
                    });
                });
        });

    row_entity
}

// ══════════════════════════════════════════════════════════════════════
// Interaction
// ══════════════════════════════════════════════════════════════════════

fn on_browse_click(
    ev: On<bevy_ui_widgets::Activate>,
    q_parent: Query<&ChildOf>,
    mut q_row: Query<&mut FileControl>,
) {
    // btn -> area -> row
    let Some(area) = q_parent.get(ev.entity).ok().map(|c| c.parent()) else {
        return;
    };
    let Some(row) = q_parent.get(area).ok().map(|c| c.parent()) else {
        return;
    };
    if let Ok(mut ctrl) = q_row.get_mut(row) {
        ctrl.pending_open = true;
    }
}

/// Launch file dialogs on a background thread.
fn launch_file_dialogs(
    mut q: Query<(Entity, &mut FileControl), Changed<FileControl>>,
    channel: Res<FileDialogChannel>,
) {
    for (entity, mut ctrl) in &mut q {
        if !ctrl.pending_open {
            continue;
        }
        ctrl.pending_open = false;

        if ctrl.dialog_in_flight {
            continue;
        }
        ctrl.dialog_in_flight = true;

        let title = ctrl.dialog_title.clone();
        let extensions = ctrl.extensions.clone();
        let tx = channel.tx.clone();

        std::thread::spawn(move || {
            let mut dialog = rfd::FileDialog::new().set_title(&title);
            if !extensions.is_empty() {
                let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
                dialog = dialog.add_filter("Files", &ext_refs);
            }
            let path = dialog.pick_file().map(|p| p.display().to_string());
            let _ = tx.send((entity, path));
        });
    }
}

/// Collect results from completed file dialogs.
fn collect_file_results(
    mut q: Query<&mut FileControl>,
    channel: Res<FileDialogChannel>,
) {
    let rx = channel.rx.lock().unwrap();
    while let Ok((entity, path)) = rx.try_recv() {
        if let Ok(mut ctrl) = q.get_mut(entity) {
            ctrl.path = path;
            ctrl.dialog_in_flight = false;
        }
    }
}

fn update_file_display(
    q: Query<(Entity, &FileControl), Changed<FileControl>>,
    q_children: Query<&Children>,
    mut q_text: Query<&mut Text, With<FilePathText>>,
    mut q_class: Query<&mut ClassList, With<FilePathText>>,
) {
    for (entity, ctrl) in &q {
        for desc in q_children.iter_descendants(entity) {
            if let Ok(mut text) = q_text.get_mut(desc) {
                if let Some(ref path) = ctrl.path {
                    // Show just the filename for brevity
                    let display = std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(path);
                    text.0 = display.to_string();
                } else {
                    text.0 = "No file selected".to_string();
                }
            }
            if let Ok(mut class) = q_class.get_mut(desc) {
                if ctrl.path.is_some() {
                    *class = ClassList::new("pane-file-path-text has-file");
                } else {
                    *class = ClassList::new("pane-file-path-text");
                }
            }
        }
    }
}

fn sync_file_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q: Query<(&PaneControlMeta, &FileControl), Changed<FileControl>>,
) {
    for (meta, ctrl) in &q {
        let value = PaneValue::String(ctrl.path.clone().unwrap_or_default());
        if store.get_raw(&meta.pane_title, &meta.label) != Some(&value) {
            store.set_raw(&meta.pane_title, &meta.label, value.clone());
            commands.trigger(PaneChanged {
                pane: meta.pane_title.clone(),
                field: meta.label.clone(),
                value,
            });
        }
    }
}

// ══════════════════════════════════════════════════════════════════════
// Builder extension
// ══════════════════════════════════════════════════════════════════════

pub trait FilePaneExt {
    /// Add a file picker with no extension filter.
    fn file(self, label: &str) -> Self;
    /// Add a file picker with extension filters.
    fn file_with_extensions(self, label: &str, extensions: &[&str]) -> Self;
}

fn file_config(extensions: &[&str]) -> ControlConfig {
    let mut config = ControlConfig::new().string("dialog_title", "Select File");
    if !extensions.is_empty() {
        config = config.string_list("extensions", extensions.iter().map(|s| s.to_string()).collect());
    }
    config
}

macro_rules! impl_file_ext {
    ($ty:ty) => {
        impl FilePaneExt for $ty {
            fn file(self, label: &str) -> Self {
                self.custom("file", label, file_config(&[]))
            }
            fn file_with_extensions(self, label: &str, extensions: &[&str]) -> Self {
                self.custom("file", label, file_config(extensions))
            }
        }
    };
}

impl_file_ext!(saddle_pane::prelude::PaneBuilder);
impl_file_ext!(saddle_pane::builder::FolderBuilder);
