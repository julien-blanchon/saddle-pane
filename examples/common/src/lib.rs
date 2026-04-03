use bevy::prelude::*;

/// Common plugin setup shared across all examples.
/// Adds `PanePlugin` and its required dependencies.
pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

/// Spawn a basic 2D reference world: camera only.
pub fn spawn_camera_2d(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Spawn a basic 3D reference world: ground plane + directional light + camera.
pub fn spawn_reference_world(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Ground
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(20.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
    ));

    // Light
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.2, 0.0)),
    ));
}
