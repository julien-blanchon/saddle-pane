//! # Basic — Minimal `#[derive(Pane)]` example
//!
//! The simplest way to use saddle-pane: derive `Pane` on a resource,
//! register it, and read values directly. No string keys, no Reflect.
//!
//! Run with: `cargo run -p saddle-pane-example-basic`

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_pane_example_common as common;

#[derive(Resource, Default, Pane)]
#[pane(title = "Game Settings")]
struct Settings {
    #[pane(slider, min = 0.0, max = 20.0, step = 0.1, tooltip = "Movement speed in units/sec")]
    speed: f32,

    #[pane(slider, min = 0.0, max = 1.0, step = 0.01)]
    volume: f32,

    enabled: bool,

    #[pane(label = "Player Name")]
    name: String,

    #[pane(color)]
    tint: Color,

    #[pane(select(options = ["Low", "Medium", "High", "Ultra"]))]
    quality: usize,

    #[pane(skip)]
    _internal: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "saddle-pane — Basic".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(common::pane_plugins())
        .register_pane::<Settings>()
        .add_systems(Startup, common::spawn_camera_2d)
        .add_systems(Update, use_settings)
        .run();
}

/// Read the resource directly — it's always in sync with the UI.
fn use_settings(settings: Res<Settings>) {
    if settings.is_changed() && !settings.is_added() {
        info!(
            "speed={:.1}, volume={:.2}, enabled={}, quality={}, name={}",
            settings.speed, settings.volume, settings.enabled, settings.quality, settings.name,
        );
    }
}
