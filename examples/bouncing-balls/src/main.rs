//! # Bouncing Balls — Visual Physics Playground
//!
//! A real-time bouncing-balls simulation with a full debug overlay.
//! Demonstrates how saddle-pane integrates into a game without polluting game code.
//!
//! Run with: `cargo run -p saddle-pane-example-bouncing-balls`
//!
//! ## What you'll see
//!
//! - Colorful balls spawning and bouncing with gravity, bounciness, and damping
//! - Four debug panels at the corners:
//!   - **Physics** (top-left) — tweak gravity, bounciness, damping in real-time
//!   - **Visuals** (top-right) — change ball color and background
//!   - **Spawner** (bottom-left) — control spawn rate, speed, size
//!   - **Stats** (bottom-right) — watch FPS, ball count, average velocity
//!
//! ## Patterns demonstrated
//!
//! 1. **Editable settings** — drag sliders, game reacts instantly
//! 2. **Monitors** — read-only stats updated every frame
//! 3. **Actions** — buttons that trigger game events (Clear All, Spawn 50)
//! 4. **External overlay** — game module has zero `saddle_pane` imports

use bevy::prelude::*;
use saddle_pane_example_common as common;

// ════════════════════════════════════════════════════════════════
// GAME MODULE — self-contained, knows nothing about saddle-pane
// ════════════════════════════════════════════════════════════════

mod game {
    use bevy::prelude::*;

    #[derive(Resource)]
    pub struct SimConfig {
        pub gravity: f32,
        pub bounciness: f32,
        pub damping: f32,
        pub bounds: Vec2,
    }

    impl Default for SimConfig {
        fn default() -> Self {
            Self {
                gravity: -400.0,
                bounciness: 0.8,
                damping: 0.99,
                bounds: Vec2::new(500.0, 300.0),
            }
        }
    }

    #[derive(Resource)]
    pub struct SpawnerConfig {
        pub rate: f32,
        pub speed_range: (f32, f32),
        pub radius_range: (f32, f32),
    }

    impl Default for SpawnerConfig {
        fn default() -> Self {
            Self {
                rate: 2.0,
                speed_range: (100.0, 300.0),
                radius_range: (4.0, 12.0),
            }
        }
    }

    #[derive(Resource)]
    pub struct VisualConfig {
        pub ball_color: Color,
        pub background_color: Color,
    }

    impl Default for VisualConfig {
        fn default() -> Self {
            Self {
                ball_color: Color::srgb(0.4, 0.7, 1.0),
                background_color: Color::srgb(0.08, 0.08, 0.1),
            }
        }
    }

    #[derive(Event, Clone)]
    pub struct SpawnBurst(pub u32);

    #[derive(Event, Clone)]
    pub struct ClearAll;

    #[derive(Component)]
    pub struct Ball {
        pub velocity: Vec2,
        pub radius: f32,
    }

    pub struct GamePlugin;

    impl Plugin for GamePlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<SimConfig>()
                .init_resource::<SpawnerConfig>()
                .init_resource::<VisualConfig>()
                .add_systems(Startup, setup)
                .add_systems(Update, (auto_spawn, physics_step, sync_background_color))
                .add_observer(on_spawn_burst)
                .add_observer(on_clear_all);
        }
    }

    fn setup(mut commands: Commands, visuals: Res<VisualConfig>) {
        commands.spawn(Camera2d);
        commands.insert_resource(ClearColor(visuals.background_color));
    }

    fn auto_spawn(
        mut commands: Commands,
        spawner: Res<SpawnerConfig>,
        time: Res<Time>,
        mut timer: Local<f32>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        visuals: Res<VisualConfig>,
    ) {
        if spawner.rate <= 0.0 {
            return;
        }
        *timer += time.delta_secs();
        let interval = 1.0 / spawner.rate;
        while *timer >= interval {
            *timer -= interval;
            spawn_ball(
                &mut commands,
                &mut meshes,
                &mut materials,
                &spawner,
                &visuals,
                &time,
            );
        }
    }

    fn spawn_ball(
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<ColorMaterial>,
        spawner: &SpawnerConfig,
        visuals: &VisualConfig,
        time: &Time,
    ) {
        let t = time.elapsed_secs();
        let hash = ((t * 1000.0) as u32).wrapping_mul(2654435761);
        let norm = |h: u32| (h as f32) / (u32::MAX as f32);

        let radius = spawner.radius_range.0
            + norm(hash) * (spawner.radius_range.1 - spawner.radius_range.0);
        let speed = spawner.speed_range.0
            + norm(hash.wrapping_mul(3)) * (spawner.speed_range.1 - spawner.speed_range.0);
        let angle = norm(hash.wrapping_mul(7)) * std::f32::consts::TAU;
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;

        commands.spawn((
            Ball { velocity, radius },
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(visuals.ball_color))),
            Transform::from_xyz(
                (norm(hash.wrapping_mul(11)) - 0.5) * 200.0,
                150.0,
                0.0,
            ),
        ));
    }

    fn physics_step(
        mut q_balls: Query<(&mut Ball, &mut Transform)>,
        config: Res<SimConfig>,
        time: Res<Time>,
    ) {
        let dt = time.delta_secs();
        let half = config.bounds;

        for (mut ball, mut tf) in &mut q_balls {
            ball.velocity.y += config.gravity * dt;
            ball.velocity *= config.damping;
            tf.translation.x += ball.velocity.x * dt;
            tf.translation.y += ball.velocity.y * dt;

            if tf.translation.x.abs() > half.x - ball.radius {
                tf.translation.x = tf.translation.x.signum() * (half.x - ball.radius);
                ball.velocity.x = -ball.velocity.x * config.bounciness;
            }
            if tf.translation.y < -half.y + ball.radius {
                tf.translation.y = -half.y + ball.radius;
                ball.velocity.y = -ball.velocity.y * config.bounciness;
            }
            if tf.translation.y > half.y - ball.radius {
                tf.translation.y = half.y - ball.radius;
                ball.velocity.y = -ball.velocity.y * config.bounciness;
            }
        }
    }

    fn sync_background_color(visuals: Res<VisualConfig>, mut clear: ResMut<ClearColor>) {
        if visuals.is_changed() {
            clear.0 = visuals.background_color;
        }
    }

    fn on_spawn_burst(
        ev: On<SpawnBurst>,
        mut commands: Commands,
        spawner: Res<SpawnerConfig>,
        visuals: Res<VisualConfig>,
        time: Res<Time>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        for _ in 0..ev.event().0 {
            spawn_ball(
                &mut commands,
                &mut meshes,
                &mut materials,
                &spawner,
                &visuals,
                &time,
            );
        }
    }

    fn on_clear_all(_ev: On<ClearAll>, mut commands: Commands, q_balls: Query<Entity, With<Ball>>) {
        for entity in &q_balls {
            commands.entity(entity).despawn();
        }
    }
}

// ════════════════════════════════════════════════════════════════
// DEBUG OVERLAY — adds panes without touching game code
// ════════════════════════════════════════════════════════════════

mod debug {
    use bevy::prelude::*;
    use saddle_pane::prelude::*;

    use super::game::{Ball, ClearAll, SimConfig, SpawnBurst, SpawnerConfig, VisualConfig};

    // ── Physics pane (editable → SimConfig) ──

    #[derive(Resource, Pane)]
    #[pane(title = "Physics", position = "top-left")]
    pub struct PhysicsPane {
        #[pane(slider, min = -800.0, max = 0.0, step = 10.0, default = -400.0,
               tooltip = "Downward acceleration")]
        gravity: f32,

        #[pane(slider, min = 0.0, max = 1.0, step = 0.01, default = 0.8,
               tooltip = "Energy retained on bounce")]
        bounciness: f32,

        #[pane(slider, min = 0.9, max = 1.0, step = 0.001, default = 0.99,
               tooltip = "Velocity multiplier per frame")]
        damping: f32,

        #[pane(slider, min = 100.0, max = 800.0, step = 10.0, default = 500.0)]
        bounds_x: f32,

        #[pane(slider, min = 100.0, max = 500.0, step = 10.0, default = 300.0)]
        bounds_y: f32,
    }

    // ── Visuals pane (editable → VisualConfig) ──

    #[derive(Resource, Pane)]
    #[pane(title = "Visuals", position = "top-right")]
    pub struct VisualsPane {
        #[pane(color)]
        ball_color: Color,

        #[pane(color, label = "Background")]
        background: Color,
    }

    impl Default for VisualsPane {
        fn default() -> Self {
            Self {
                ball_color: Color::srgb(0.4, 0.7, 1.0),
                background: Color::srgb(0.08, 0.08, 0.1),
            }
        }
    }

    // ── Spawner pane (mixed: editable + monitors + actions) ──

    #[derive(Resource, Pane)]
    #[pane(title = "Spawner", position = "bottom-left")]
    pub struct SpawnerPane {
        #[pane(slider, min = 0.0, max = 20.0, step = 0.5, default = 2.0,
               tooltip = "Balls per second")]
        rate: f32,

        #[pane(slider, min = 50.0, max = 500.0, step = 10.0, default = 100.0)]
        min_speed: f32,

        #[pane(slider, min = 100.0, max = 600.0, step = 10.0, default = 300.0)]
        max_speed: f32,

        #[pane(slider, min = 2.0, max = 20.0, step = 0.5, default = 4.0)]
        min_radius: f32,

        #[pane(slider, min = 4.0, max = 30.0, step = 0.5, default = 12.0)]
        max_radius: f32,

        #[pane(monitor, label = "Ball Count")]
        ball_count: u32,
    }

    // ── Stats pane (all monitors) ──

    #[derive(Resource, Default, Pane)]
    #[pane(title = "Stats", position = "bottom-right")]
    pub struct StatsPane {
        #[pane(monitor)]
        fps: f32,

        #[pane(monitor, label = "Avg Velocity")]
        avg_velocity: f32,

        #[pane(monitor, label = "Total Entities")]
        total_entities: u32,

        #[pane(monitor, label = "Frame")]
        frame: u64,
    }

    // ── Plugin ──

    pub struct DebugPlugin;

    impl Plugin for DebugPlugin {
        fn build(&self, app: &mut App) {
            app.register_pane::<PhysicsPane>()
                .register_pane::<VisualsPane>()
                .register_pane::<SpawnerPane>()
                .register_pane::<StatsPane>()
                .add_systems(
                    Update,
                    (
                        sync_physics_to_game,
                        sync_visuals_to_game,
                        sync_spawner_to_game,
                        update_stats,
                        update_spawner_monitors,
                    ),
                )
                .add_observer(on_button);
        }
    }

    fn sync_physics_to_game(pane: Res<PhysicsPane>, mut config: ResMut<SimConfig>) {
        if pane.is_changed() && !pane.is_added() {
            config.gravity = pane.gravity;
            config.bounciness = pane.bounciness;
            config.damping = pane.damping;
            config.bounds = Vec2::new(pane.bounds_x, pane.bounds_y);
        }
    }

    fn sync_visuals_to_game(pane: Res<VisualsPane>, mut config: ResMut<VisualConfig>) {
        if pane.is_changed() && !pane.is_added() {
            config.ball_color = pane.ball_color;
            config.background_color = pane.background;
        }
    }

    fn sync_spawner_to_game(pane: Res<SpawnerPane>, mut config: ResMut<SpawnerConfig>) {
        if pane.is_changed() && !pane.is_added() {
            config.rate = pane.rate;
            config.speed_range = (pane.min_speed, pane.max_speed);
            config.radius_range = (pane.min_radius, pane.max_radius);
        }
    }

    fn update_spawner_monitors(mut pane: ResMut<SpawnerPane>, q_balls: Query<(), With<Ball>>) {
        let count = q_balls.iter().count() as u32;
        if pane.ball_count != count {
            pane.ball_count = count;
        }
    }

    fn update_stats(
        mut pane: ResMut<StatsPane>,
        q_balls: Query<&Ball>,
        q_all: Query<Entity>,
        time: Res<Time>,
        diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    ) {
        if let Some(fps) = diagnostics
            .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.smoothed())
        {
            pane.fps = fps as f32;
        }

        let (sum, count) = q_balls
            .iter()
            .fold((0.0_f32, 0u32), |(sum, count), ball| {
                (sum + ball.velocity.length(), count + 1)
            });
        pane.avg_velocity = if count > 0 { sum / count as f32 } else { 0.0 };
        pane.total_entities = q_all.iter().count() as u32;
        pane.frame = (time.elapsed_secs() * 60.0) as u64;
    }

    fn on_button(ev: On<PaneButtonPressed>, mut commands: Commands) {
        match ev.event().label.as_str() {
            "Spawn 50" => commands.trigger(SpawnBurst(50)),
            "Clear All" => commands.trigger(ClearAll),
            _ => {}
        }
    }
}

// ════════════════════════════════════════════════════════════════
// MAIN — compose game + debug overlay
// ════════════════════════════════════════════════════════════════

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "saddle-pane — Bouncing Balls".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(common::pane_plugins())
        .add_plugins(game::GamePlugin)
        .add_plugins(debug::DebugPlugin)
        .run();
}
