use bevy::prelude::*;
use bevy_flair::FlairPlugin;
use bevy_input_focus::InputDispatchPlugin;
use saddle_pane::prelude::*;
use saddle_pane_bezier::{BezierPaneExt, PaneBezierPlugin};
use saddle_pane_button_grid::{ButtonGridPaneExt, PaneButtonGridPlugin};
use saddle_pane_file::{FilePaneExt, PaneFilePlugin};
use saddle_pane_interval::PaneIntervalPlugin;
use saddle_pane_radio_grid::{PaneRadioGridPlugin, RadioGridPaneExt};
use saddle_pane_vector2::{PaneVector2Plugin, Vector2PaneExt};
use bevy_ui_widgets::UiWidgetsPlugins;

#[cfg(feature = "e2e")]
mod scenarios;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "saddle-pane — Lab".to_string(),
            resolution: bevy::window::WindowResolution::new(800, 600),
            ..default()
        }),
        ..default()
    }));

    app.add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default());
    app.add_plugins(FlairPlugin);
    app.add_plugins(InputDispatchPlugin);
    app.add_plugins(UiWidgetsPlugins);
    app.add_plugins(bevy_input_focus::tab_navigation::TabNavigationPlugin);
    app.add_plugins(PanePlugin);
    app.add_plugins(PaneIntervalPlugin);
    app.add_plugins(PaneButtonGridPlugin);
    app.add_plugins(PaneRadioGridPlugin);
    app.add_plugins(PaneVector2Plugin);
    app.add_plugins(PaneBezierPlugin);
    app.add_plugins(PaneFilePlugin);

    #[cfg(feature = "brp")]
    {
        app.add_plugins(bevy::remote::RemotePlugin::default());
        app.add_plugins(bevy::remote::http::RemoteHttpPlugin::default());
        app.add_plugins(bevy_brp_extras::BrpExtrasPlugin);
    }

    #[cfg(feature = "e2e")]
    {
        app.add_plugins(saddle_bevy_e2e::E2EPlugin);

        let args: Vec<String> = std::env::args().collect();
        let scenario_name = args.iter().skip(1).find(|a| !a.starts_with('-')).cloned();
        let handoff = args.iter().any(|a| a == "--handoff");

        if let Some(ref name) = scenario_name {
            if let Some(mut scenario) = scenarios::scenario_by_name(name) {
                if handoff {
                    scenario.actions.push(saddle_bevy_e2e::action::Action::Handoff);
                }
                saddle_bevy_e2e::init_scenario(&mut app, scenario);
            } else {
                error!(
                    "[saddle-pane-lab] Unknown scenario '{name}'. Available: {:?}",
                    scenarios::list_scenarios()
                );
            }
        }

        // E2E test helpers — programmatically trigger interactions
        if scenario_name.as_deref() == Some("dropdown_open") {
            app.add_systems(Update, e2e_open_dropdown);
        }
        if scenario_name.as_deref() == Some("reset_all") {
            app.add_systems(Update, e2e_modify_then_reset);
        }
        if scenario_name.as_deref() == Some("color_picker_open") {
            app.add_systems(Update, e2e_open_color_picker);
        }
        if scenario_name.as_deref() == Some("theme_light") {
            app.add_systems(Update, e2e_switch_theme_light);
        }
        if scenario_name.as_deref() == Some("store_bidirectional") {
            app.add_systems(Update, e2e_store_set);
        }
        if scenario_name.as_deref() == Some("save_load") {
            app.add_systems(Update, e2e_save_load);
        }

        // Tab-switching scenarios
        let tab_map: std::collections::HashMap<&str, usize> = [
            ("tab_physics", 1),
            ("tab_render", 2),
            ("tab_grids", 4),
            ("tab_files", 5),
            ("tab_status", 7),
        ].into_iter().collect();
        if let Some(&tab_idx) = scenario_name.as_deref().and_then(|n| tab_map.get(n)) {
            app.insert_resource(E2ETabSwitch(tab_idx));
            app.add_systems(Update, e2e_switch_tab);
        }
    }

    app.add_systems(Startup, setup);
    app.add_systems(Update, (update_monitors, show_loaded_image));
    app.add_observer(on_button);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    PaneBuilder::new("All Controls")
        .tab("General", |p| {
            p.slider("Speed", Slider::new(0.0..=10.0, 5.0).step(0.1)
                    .tooltip("Movement speed in units/sec")
                    .icon(saddle_pane::icons::ICON_ACTIVITY))
                .slider("Volume", Slider::new(0.0..=1.0, 0.8).step(0.01)
                    .tooltip("Master volume level"))
                .slider("Opacity", Slider::new(0.0..=1.0, 1.0).step(0.01)
                    .icon(saddle_pane::icons::ICON_EYE))
                .toggle("Enabled", true)
                .toggle_opts("Debug Mode", Toggle::new(false)
                    .icon(saddle_pane::icons::ICON_BUG))
                .separator()
                .number("Score", Number::new(100.0))
                .number("Lives", Number::new(3.0).step(1.0))
                .text("Name", "Hero")
                .text("Tag", "player_01")
                .select("Quality", &["Low", "Medium", "High", "Ultra"], 2)
                .select("Mode", &["Easy", "Normal", "Hard"], 1)
                .color("Ambient", Color::srgb(0.2, 0.3, 0.8))
                .color("Fog", Color::srgb(0.5, 0.5, 0.6))
        })
        .tab("Physics", |p| {
            p.slider("Gravity", Slider::new(-20.0..=0.0, -9.81).step(0.1))
                .slider("Friction", Slider::new(0.0..=1.0, 0.3))
                .slider("Bounciness", Slider::new(0.0..=1.0, 0.5).step(0.05))
                .toggle("Enabled", true)
                .number("Max Velocity", Number::new(50.0).step(5.0))
                .interval("Spawn Range", 0.0..=100.0, 20.0..=80.0)
                .interval("Temp Range", -40.0..=60.0, -10.0..=35.0)
                .separator()
                .vector2("Force", (0.0, -9.81))
                .vector2_bounded(
                    "Spawn Pos",
                    -50.0..=50.0,
                    0.0..=100.0,
                    (0.0, 10.0),
                )
        })
        .tab("Render", |p| {
            p.toggle("Shadows", true)
                .toggle("SSAO", false)
                .toggle("Bloom", true)
                .slider("Exposure", Slider::new(-3.0..=3.0, 0.0).step(0.1))
                .slider("Gamma", Slider::new(0.5..=3.0, 2.2).step(0.01))
                .color_opts("Sky Color", ColorPicker::new(Color::srgb(0.4, 0.6, 0.9))
                    .icon(saddle_pane::icons::ICON_PALETTE))
                .select_opts("Tone Map", SelectMenu::new(&["None", "Reinhard", "ACES", "AgX"], 3)
                    .icon(saddle_pane::icons::ICON_SETTINGS))
                .separator()
                .bezier("Ease Curve")
        })
        .tab("Audio", |p| {
            p.slider("Master", Slider::new(0.0..=1.0, 1.0).step(0.01))
                .slider("Music", Slider::new(0.0..=1.0, 0.7).step(0.01))
                .slider("SFX", Slider::new(0.0..=1.0, 0.9).step(0.01))
                .toggle("Mute", false)
        })
        .tab("Grids", |p| {
            p.radio_grid(
                    "Shape",
                    &["Cube", "Sphere", "Cylinder", "Torus", "Capsule", "Cone"],
                    0,
                )
                .checkbox_grid(
                    "Effects",
                    &["Bloom", "SSAO", "SSR", "DoF", "Motion Blur", "TAA"],
                    &[0, 5],
                )
                .multi_grid(
                    "Layers",
                    &["Base", "Detail", "Overlay", "FX"],
                    2,
                    &[0],
                )
                .separator()
                .button_grid("Actions", &["Spawn", "Delete", "Clone", "Reset", "Undo", "Redo"])
                .button_grid_columns("Tools", &["Select", "Move", "Rotate", "Scale"], 4)
        })
        .tab("Files", |p| {
            p.file("Texture")
                .file_with_extensions("Model", &["obj", "gltf", "glb"])
                .file_with_extensions("Audio", &["ogg", "wav", "mp3"])
        })
        .tab("Monitor", |p| {
            p.monitor("FPS", Monitor::text("\u{2014}"))
                .monitor("Console", Monitor::log(8))
                .monitor("CPU", Monitor::graph(0.0..=100.0, 64))
        })
        .tab("Debug", |p| {
            p.toggle("Wireframe", false)
                .toggle("Gizmos", true)
                .toggle("Colliders", false)
                .select("Log Level", &["Error", "Warn", "Info", "Debug", "Trace"], 2)
                .text("Filter", "")
        })
        .footer(|f| {
            f.button("Reset All")
                .button("Save")
                .button("Load")
        })
        .spawn(&mut commands);
}

/// Live-update monitor controls each frame.
fn update_monitors(
    time: Res<Time>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut q_monitors: Query<(&saddle_pane::controls::PaneControlMeta, &mut MonitorControl)>,
    mut q_logs: Query<(&saddle_pane::controls::PaneControlMeta, &mut MonitorLogControl)>,
    mut q_graphs: Query<(&saddle_pane::controls::PaneControlMeta, &mut MonitorGraphControl)>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Update FPS monitor
    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    for (meta, mut monitor) in &mut q_monitors {
        if meta.label == "FPS" {
            monitor.value = format!("{fps:.1}");
        }
    }

    // Push log line every 60 frames
    if (*frame_count).is_multiple_of(60) {
        let elapsed = time.elapsed_secs();
        for (meta, mut log) in &mut q_logs {
            if meta.label == "Console" {
                log.push(format!("[{elapsed:.1}s] tick #{}", *frame_count));
            }
        }
    }

    // Push graph value every 4 frames (simulated CPU usage)
    if (*frame_count).is_multiple_of(4) {
        let t = time.elapsed_secs();
        let simulated_cpu = 30.0 + 20.0 * (t * 0.5).sin() + 10.0 * (t * 1.7).cos();
        for (meta, mut graph) in &mut q_graphs {
            if meta.label == "CPU" {
                graph.push(simulated_cpu);
            }
        }
    }
}

fn on_button(ev: On<PaneButtonPressed>, mut store: ResMut<PaneStore>) {
    let label = &ev.event().label;
    info!("Button: {}/{}", ev.event().pane, label);

    if label == "Save" {
        let json = store.save_json();
        let path = "pane_state.json";
        match std::fs::write(path, &json) {
            Ok(()) => info!("Saved pane state to {path} ({} bytes)", json.len()),
            Err(e) => warn!("Failed to save pane state: {e}"),
        }
    } else if label == "Load" {
        let path = "pane_state.json";
        match std::fs::read_to_string(path) {
            Ok(json) => match store.load_json(&json) {
                Ok(count) => info!("Loaded {count} values from {path}"),
                Err(e) => warn!("Failed to parse pane state: {e}"),
            },
            Err(e) => warn!("Failed to read {path}: {e}"),
        }
    }
}

/// Marker for the image preview sprite.
#[derive(Component)]
struct ImagePreview;

/// Watch for Texture file changes and display the loaded image on screen.
fn show_loaded_image(
    q_file: Query<
        (&saddle_pane::controls::PaneControlMeta, &saddle_pane_file::FileControl),
        Changed<saddle_pane_file::FileControl>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
    existing: Query<Entity, With<ImagePreview>>,
) {
    for (meta, ctrl) in &q_file {
        if meta.label != "Texture" {
            continue;
        }
        let Some(ref path) = ctrl.path else {
            continue;
        };

        // Load image from arbitrary filesystem path
        let Ok(bytes) = std::fs::read(path) else {
            warn!("Failed to read file: {path}");
            continue;
        };
        let Ok(dyn_img) = image::load_from_memory(&bytes) else {
            warn!("Failed to decode image: {path}");
            continue;
        };

        let bevy_image = Image::from_dynamic(
            dyn_img,
            true,
            bevy::asset::RenderAssetUsages::MAIN_WORLD
                | bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        let handle = images.add(bevy_image);

        // Despawn any existing preview
        for entity in &existing {
            commands.entity(entity).despawn();
        }

        commands.spawn((
            Sprite {
                image: handle,
                ..default()
            },
            Transform::from_xyz(-200.0, 0.0, 0.0).with_scale(Vec3::splat(0.3)),
            ImagePreview,
        ));
        info!("Loaded image preview: {path}");
    }
}

// ── E2E test helpers ──

/// Programmatically open the Quality dropdown at frame 100.
#[cfg(feature = "e2e")]
fn e2e_open_dropdown(
    mut frame: Local<u32>,
    mut q_select: Query<(
        &saddle_pane::controls::PaneControlMeta,
        &mut saddle_pane::controls::select::SelectOpen,
    )>,
) {
    *frame += 1;
    if *frame == 100 {
        for (meta, mut open) in &mut q_select {
            if meta.label == "Quality" {
                open.0 = true;
            }
        }
    }
}

/// Programmatically open the color picker at frame 100.
#[cfg(feature = "e2e")]
fn e2e_open_color_picker(
    mut frame: Local<u32>,
    mut q_picker: Query<(
        &saddle_pane::controls::PaneControlMeta,
        &mut saddle_pane::controls::color_picker::ColorPickerOpen,
    )>,
) {
    *frame += 1;
    if *frame == 100 {
        for (meta, mut open) in &mut q_picker {
            if meta.label == "Ambient" {
                open.0 = true;
                info!("[e2e] Opened color picker for Ambient");
            }
        }
    }
}

#[cfg(feature = "e2e")]
#[derive(Resource)]
struct E2ETabSwitch(usize);

/// Switch to a specific tab at frame 80 by triggering Activate on the target tab button.
#[cfg(feature = "e2e")]
fn e2e_switch_tab(
    mut frame: Local<u32>,
    tab_switch: Res<E2ETabSwitch>,
    q_children: Query<&Children>,
    q_text: Query<&Text>,
    q_class: Query<(Entity, &bevy_flair::prelude::ClassList), With<Interaction>>,
    mut commands: Commands,
) {
    *frame += 1;
    if *frame != 80 {
        return;
    }

    let tab_names = [
        "General", "Physics", "Render", "Audio", "Grids", "Files", "Monitor", "Debug",
    ];
    let target_name = tab_names.get(tab_switch.0).copied().unwrap_or("General");

    // Find the tab button entity matching the target name
    for (entity, class) in q_class.iter() {
        let class_str = format!("{class:?}");
        if class_str.contains("pane-tab-button") {
            if let Ok(children) = q_children.get(entity) {
                for child in children.iter() {
                    if let Ok(text) = q_text.get(child) {
                        if text.0 == target_name {
                            commands.trigger(bevy_ui_widgets::Activate { entity });
                            info!("[e2e] Switched to tab: {target_name}");
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// Modify controls at frame 100, trigger reset at frame 130.
#[cfg(feature = "e2e")]
fn e2e_modify_then_reset(
    mut frame: Local<u32>,
    mut q_sliders: Query<(
        &saddle_pane::controls::PaneControlMeta,
        &mut saddle_pane::controls::slider::SliderControl,
    )>,
    mut commands: Commands,
) {
    *frame += 1;
    if *frame == 100 {
        for (meta, mut ctrl) in &mut q_sliders {
            if meta.label == "Speed" {
                ctrl.value = 1.0;
            }
            if meta.label == "Volume" {
                ctrl.value = 0.1;
            }
        }
    }
    if *frame == 130 {
        commands.trigger(PaneButtonPressed {
            pane: "All Controls".to_string(),
            label: "Reset All".to_string(),
        });
    }
}

/// Save/load cycle: modify → save → reset → load.
#[cfg(feature = "e2e")]
fn e2e_save_load(
    mut frame: Local<u32>,
    mut store: ResMut<saddle_pane::prelude::PaneStore>,
    mut commands: Commands,
) {
    *frame += 1;
    match *frame {
        100 => {
            store.set("All Controls", "Speed", 2.0_f64);
            info!("[e2e] Set Speed to 2.0");
        }
        110 => {
            let json = store.save_json();
            std::fs::write("pane_state.json", &json).unwrap();
            info!("[e2e] Saved state ({} bytes)", json.len());
        }
        130 => {
            commands.trigger(PaneButtonPressed {
                pane: "All Controls".to_string(),
                label: "Reset All".to_string(),
            });
            info!("[e2e] Reset All triggered");
        }
        150 => {
            let json = std::fs::read_to_string("pane_state.json").unwrap();
            let count = store.load_json(&json).unwrap();
            info!("[e2e] Loaded {count} values");
        }
        _ => {}
    }
}

/// Set Speed to 2.0 via PaneStore at frame 100.
#[cfg(feature = "e2e")]
fn e2e_store_set(
    mut frame: Local<u32>,
    mut store: ResMut<saddle_pane::prelude::PaneStore>,
) {
    *frame += 1;
    if *frame == 100 {
        store.set("All Controls", "Speed", 2.0_f64);
        info!("[e2e] Set Speed to 2.0 via store.set()");
    }
}

/// Switch to Light theme at frame 100.
#[cfg(feature = "e2e")]
fn e2e_switch_theme_light(
    mut frame: Local<u32>,
    mut theme: ResMut<saddle_pane::prelude::PaneTheme>,
) {
    *frame += 1;
    if *frame == 100 {
        theme.active = saddle_pane::prelude::PaneThemePreset::Light;
        info!("[e2e] Switched to Light theme");
    }
}
