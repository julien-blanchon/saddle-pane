//! # Game Debug — Three Debug Patterns
//!
//! Demonstrates the three main patterns for game debugging with saddle-pane:
//!
//! 1. **Editable settings** — tweak physics, graphics in real-time
//! 2. **Monitor-only stats** — watch FPS, entity count without editing
//! 3. **Mixed panels** — some fields editable, others display-only
//!
//! Run with: `cargo run -p saddle-pane-example-game-debug`

use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_pane_example_common as common;

// ────────────────────────────────────────────────────
// Pattern 1: Editable Settings
// Game reads these directly. User tweaks them via the pane.
// ────────────────────────────────────────────────────

#[derive(Resource, Pane)]
#[pane(title = "Physics")]
struct PhysicsSettings {
    #[pane(slider, min = -20.0, max = 0.0, step = 0.1, default = -9.81)]
    gravity: f32,

    #[pane(slider, min = 0.0, max = 1.0, step = 0.01, default = 0.3)]
    friction: f32,

    #[pane(slider, min = 0.0, max = 2.0, step = 0.01, default = 0.5)]
    bounciness: f32,

    #[pane(default = true)]
    simulate: bool,
}

// ────────────────────────────────────────────────────
// Pattern 2: Monitor-Only Stats
// Game writes these every frame. Pane just displays them.
// ────────────────────────────────────────────────────

#[derive(Resource, Default, Pane)]
#[pane(title = "Stats")]
struct GameStats {
    #[pane(monitor)]
    fps: f32,

    #[pane(monitor)]
    entity_count: u32,

    #[pane(monitor)]
    frame: u64,
}

// ────────────────────────────────────────────────────
// Pattern 3: Mixed — some editable, some monitor-only
// ────────────────────────────────────────────────────

#[derive(Resource, Default, Pane)]
#[pane(title = "Player")]
struct PlayerDebug {
    #[pane(slider, min = 1.0, max = 20.0, step = 0.5, tooltip = "Movement speed")]
    speed: f32,

    #[pane(slider, min = 1.0, max = 20.0, step = 0.5)]
    jump_force: f32,

    #[pane(tooltip = "Infinite health")]
    god_mode: bool,

    #[pane(monitor, label = "Health")]
    health_display: f32,

    #[pane(monitor, label = "Position")]
    position_display: String,

    #[pane(skip)]
    _health: f32,
    #[pane(skip)]
    _pos_x: f32,
    #[pane(skip)]
    _pos_y: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "saddle-pane — Game Debug".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(common::pane_plugins())
        .register_pane::<PhysicsSettings>()
        .register_pane::<GameStats>()
        .register_pane::<PlayerDebug>()
        .add_systems(Startup, common::spawn_camera_2d)
        .add_systems(Update, (update_stats, simulate_player, apply_physics))
        .run();
}

fn update_stats(
    mut stats: ResMut<GameStats>,
    time: Res<Time>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    entities: Query<Entity>,
) {
    if let Some(fps) = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    {
        stats.fps = fps as f32;
    }
    stats.entity_count = entities.iter().count() as u32;
    stats.frame = (time.elapsed_secs() * 60.0) as u64;
}

fn simulate_player(mut player: ResMut<PlayerDebug>, time: Res<Time>) {
    let t = time.elapsed_secs();
    player._pos_x = (t * player.speed * 0.1).sin() * 100.0;
    player._pos_y = (t * player.speed * 0.07).cos() * 50.0;

    if !player.god_mode {
        player._health = (100.0 - (t * 2.0) % 100.0).max(0.0);
    } else {
        player._health = 100.0;
    }

    player.health_display = player._health;
    player.position_display = format!("({:.1}, {:.1})", player._pos_x, player._pos_y);
}

fn apply_physics(physics: Res<PhysicsSettings>) {
    if physics.is_changed() && !physics.is_added() {
        if physics.simulate {
            info!(
                "Physics: gravity={:.2}, friction={:.2}, bounce={:.2}",
                physics.gravity, physics.friction, physics.bounciness,
            );
        } else {
            info!("Physics simulation paused");
        }
    }
}
