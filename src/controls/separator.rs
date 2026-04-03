use bevy::prelude::*;
use bevy_flair::prelude::ClassList;
use bevy_flair::style::components::NodeStyleSheet;

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/separator.css";

/// Marker component for separator controls.
#[derive(Component, Clone, Debug, Default)]
pub struct SeparatorControl;

/// Spawn separator UI as children of `parent`.
pub(crate) fn spawn_separator_ui(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
) -> Entity {
    parent
        .spawn((
            Node::default(),
            ClassList::new("pane-separator"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            SeparatorControl,
        ))
        .id()
}
