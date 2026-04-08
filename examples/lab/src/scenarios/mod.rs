use bevy::prelude::*;
use saddle_bevy_e2e::action::Action;
use saddle_bevy_e2e::actions::assertions;
use saddle_bevy_e2e::scenario::Scenario;

use saddle_pane::controls::PaneControlMeta;
use saddle_pane::controls::color::ColorControl;
use saddle_pane::controls::select::SelectControl;
use saddle_pane::controls::slider::SliderControl;
use saddle_pane::controls::toggle::ToggleControl;
use saddle_pane::layout::PaneRoot;
use saddle_pane::prelude::{PaneStore, PaneTheme, PaneThemePreset};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "all_controls_visible",
        "dropdown_open",
        "reset_all",
        "color_picker_open",
        "tab_physics",
        "tab_grids",
        "tab_render",
        "tab_files",
        "tab_status",
        "theme_light",
        "store_bidirectional",
        "save_load",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "all_controls_visible" => Some(all_controls_visible()),
        "dropdown_open" => Some(dropdown_open()),
        "reset_all" => Some(reset_all()),
        "color_picker_open" => Some(color_picker_open()),
        "tab_physics" => Some(tab_screenshot("tab_physics")),
        "tab_grids" => Some(tab_screenshot("tab_grids")),
        "tab_render" => Some(tab_screenshot("tab_render")),
        "tab_files" => Some(tab_screenshot("tab_files")),
        "tab_status" => Some(tab_screenshot("tab_status")),
        "theme_light" => Some(theme_light()),
        "store_bidirectional" => Some(store_bidirectional()),
        "save_load" => Some(save_load()),
        _ => None,
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Build an `Action::Custom` that acts as a soft assertion.
///
/// Logs [PASS] or [FAIL] and records into [`assertions::AssertionTracker`], matching the
/// behaviour of `assertions::custom` but with full `&mut World` access so that multi-component
/// ECS queries (e.g. `(&PaneControlMeta, &SliderControl)`) work correctly.
macro_rules! assert_world {
    ($label:expr, $world:ident => $body:expr) => {
        Action::Custom(Box::new(|$world: &mut World| {
            use saddle_bevy_e2e::actions::assertions::AssertionTracker;
            let passed: bool = $body;
            let label: &str = $label;
            // Record into tracker
            if !$world.contains_resource::<AssertionTracker>() {
                $world.insert_resource(AssertionTracker::default());
            }
            {
                let mut tracker = $world.resource_mut::<AssertionTracker>();
                if passed {
                    tracker.passed += 1;
                } else {
                    tracker.failed += 1;
                    tracker.failures.push(label.to_string());
                }
            }
            if passed {
                info!("[e2e] [PASS] {}", label);
            } else {
                info!("[e2e] [FAIL] {}", label);
            }
        }))
    };
}

// ── smoke_launch ─────────────────────────────────────────────────────────────

fn smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot and verify at least one PaneRoot entity and core resources are spawned")
        .then(Action::WaitFrames(90))
        // At least one pane root exists
        .then(assertions::entity_exists::<PaneRoot>("PaneRoot entity exists"))
        // The PaneStore resource is present
        .then(assertions::resource_exists::<PaneStore>("PaneStore resource exists"))
        // Slider controls are present (General tab has Speed, Volume, Opacity)
        .then(assert_world!("at least one SliderControl spawned", w => {
            let mut q = w.query::<&SliderControl>();
            q.iter(w).count() > 0
        }))
        // Toggle controls are present (General tab has Enabled + Debug Mode)
        .then(assert_world!("at least one ToggleControl spawned", w => {
            let mut q = w.query::<&ToggleControl>();
            q.iter(w).count() > 0
        }))
        .then(assertions::log_summary("smoke_launch summary"))
        .then(Action::Screenshot("smoke_launch".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── all_controls_visible ─────────────────────────────────────────────────────

fn all_controls_visible() -> Scenario {
    Scenario::builder("all_controls_visible")
        .description("Verify that all expected control types appear and initial values are correct")
        .then(Action::WaitFrames(90))
        // At least 3 slider controls (Speed, Volume, Opacity in General)
        .then(assert_world!("at least 3 SliderControls exist", w => {
            let mut q = w.query::<&SliderControl>();
            q.iter(w).count() >= 3
        }))
        // At least 2 toggle controls (Enabled + Debug Mode in General)
        .then(assert_world!("at least 2 ToggleControls exist", w => {
            let mut q = w.query::<&ToggleControl>();
            q.iter(w).count() >= 2
        }))
        // At least 1 dropdown (Quality in General)
        .then(assert_world!("at least 1 SelectControl exists", w => {
            let mut q = w.query::<&SelectControl>();
            q.iter(w).count() >= 1
        }))
        // At least 1 color control (Ambient in General)
        .then(assert_world!("at least 1 ColorControl exists", w => {
            let mut q = w.query::<&ColorControl>();
            q.iter(w).count() >= 1
        }))
        // Speed slider has correct initial value (5.0)
        .then(assert_world!("Speed slider initialised to 5.0", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Speed" && (ctrl.value - 5.0).abs() < 1e-6)
        }))
        // Volume slider has correct initial value (0.8)
        .then(assert_world!("Volume slider initialised to 0.8", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Volume" && (ctrl.value - 0.8).abs() < 1e-6)
        }))
        // Quality dropdown has the correct initial selection (index 2 = "High")
        .then(assert_world!("Quality dropdown initialised to index 2", w => {
            let mut q = w.query::<(&PaneControlMeta, &SelectControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Quality" && ctrl.value == 2)
        }))
        .then(assertions::log_summary("all_controls_visible summary"))
        .then(Action::Screenshot("all_controls_01".into()))
        .then(Action::WaitFrames(60))
        .then(Action::Screenshot("all_controls_02".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── dropdown_open ─────────────────────────────────────────────────────────────

fn dropdown_open() -> Scenario {
    Scenario::builder("dropdown_open")
        .description(
            "Verify Quality SelectControl exists with 4 options, then open it and screenshot",
        )
        .then(Action::WaitFrames(90))
        // Confirm Quality dropdown exists before opening
        .then(assert_world!("Quality SelectControl exists", w => {
            let mut q = w.query::<(&PaneControlMeta, &SelectControl)>();
            q.iter(w).any(|(meta, _)| meta.label == "Quality")
        }))
        // Quality dropdown has the correct option count
        .then(assert_world!("Quality dropdown has 4 options", w => {
            let mut q = w.query::<(&PaneControlMeta, &SelectControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Quality" && ctrl.options.len() == 4)
        }))
        .then(Action::Screenshot("dropdown_before".into()))
        // Wait for the e2e_open_dropdown system to fire at frame 100
        .then(Action::WaitFrames(30))
        .then(assertions::log_summary("dropdown_open summary"))
        .then(Action::Screenshot("dropdown_open".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── color_picker_open ────────────────────────────────────────────────────────

fn color_picker_open() -> Scenario {
    Scenario::builder("color_picker_open")
        .description(
            "Verify Ambient ColorControl with expected initial color, then open picker and \
             screenshot",
        )
        .then(Action::WaitFrames(90))
        // Confirm Ambient ColorControl exists
        .then(assert_world!("Ambient ColorControl exists", w => {
            let mut q = w.query::<(&PaneControlMeta, &ColorControl)>();
            q.iter(w).any(|(meta, _)| meta.label == "Ambient")
        }))
        // Ambient color is initialised to approximately srgb(0.2, 0.3, 0.8)
        .then(assert_world!("Ambient color has expected initial value", w => {
            let mut q = w.query::<(&PaneControlMeta, &ColorControl)>();
            q.iter(w).any(|(meta, ctrl)| {
                if meta.label != "Ambient" {
                    return false;
                }
                let srgba = ctrl.value.to_srgba();
                (srgba.red - 0.2).abs() < 0.05
                    && (srgba.green - 0.3).abs() < 0.05
                    && (srgba.blue - 0.8).abs() < 0.05
            })
        }))
        .then(Action::Screenshot("color_picker_before".into()))
        // Wait for e2e_open_color_picker to fire at frame 100
        .then(Action::WaitFrames(30))
        .then(assertions::log_summary("color_picker_open summary"))
        .then(Action::Screenshot("color_picker_open".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── tab_screenshot (generic helper used by four tab scenarios) ────────────────

fn tab_screenshot(name: &str) -> Scenario {
    let tab_label = name.strip_prefix("tab_").unwrap_or(name);
    let tab_label_cap: String = tab_label
        .chars()
        .enumerate()
        .map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c })
        .collect();

    let summary_label = format!("{name} summary");
    Scenario::builder(name)
        .description(format!(
            "Switch to the {tab_label_cap} tab and verify pane root and store survive"
        ))
        .then(Action::WaitFrames(90))
        // PaneRoot entity still exists after tab switch
        .then(assertions::entity_exists::<PaneRoot>("PaneRoot survives tab switch"))
        // PaneStore is still populated with at least the Speed key
        .then(assertions::resource_satisfies::<PaneStore>(
            "PaneStore has Speed key after tab switch",
            |store| store.contains("All Controls", "Speed"),
        ))
        .then(Action::Screenshot(format!("{name}_01")))
        // Wait for live-updating read-only controls (monitor graphs, FPS)
        .then(Action::WaitFrames(30))
        .then(assertions::log_summary(summary_label.as_str()))
        .then(Action::Screenshot(format!("{name}_02")))
        .then(Action::WaitFrames(1))
        .build()
}

// ── theme_light ───────────────────────────────────────────────────────────────

fn theme_light() -> Scenario {
    Scenario::builder("theme_light")
        .description(
            "Verify PaneTheme starts as Dark, then switch to Light via e2e helper and assert the \
             resource reflects the change",
        )
        .then(Action::WaitFrames(90))
        // Default theme should be Dark
        .then(assertions::resource_satisfies::<PaneTheme>(
            "PaneTheme starts as Dark",
            |theme| theme.active == PaneThemePreset::Dark,
        ))
        .then(Action::Screenshot("theme_dark_before".into()))
        // The e2e_switch_theme_light system switches to Light at frame 100 (we're at ~90, wait 20)
        .then(Action::WaitFrames(20))
        // Theme should now be Light
        .then(assertions::resource_satisfies::<PaneTheme>(
            "PaneTheme switched to Light",
            |theme| theme.active == PaneThemePreset::Light,
        ))
        // Pane root still exists after theme switch
        .then(assertions::entity_exists::<PaneRoot>("PaneRoot survives theme switch"))
        .then(assertions::log_summary("theme_light summary"))
        .then(Action::Screenshot("theme_light_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── store_bidirectional ───────────────────────────────────────────────────────

fn store_bidirectional() -> Scenario {
    Scenario::builder("store_bidirectional")
        .description(
            "Set Speed to 2.0 via PaneStore.set(), then assert the store and the SliderControl \
             component both reflect the new value",
        )
        .then(Action::WaitFrames(90))
        // Speed starts at default (5.0)
        .then(assertions::resource_satisfies::<PaneStore>(
            "Speed initialised to 5.0 in store",
            |store| {
                store
                    .try_get::<f64>("All Controls", "Speed")
                    .is_some_and(|v| (v - 5.0).abs() < 1e-4)
            },
        ))
        .then(Action::Screenshot("store_before".into()))
        // e2e_store_set fires at frame 100; we're at ~90 so wait 20 more frames
        .then(Action::WaitFrames(20))
        // PaneStore should now reflect 2.0
        .then(assertions::resource_satisfies::<PaneStore>(
            "PaneStore Speed updated to 2.0",
            |store| {
                store
                    .try_get::<f64>("All Controls", "Speed")
                    .is_some_and(|v| (v - 2.0).abs() < 1e-4)
            },
        ))
        // SliderControl component should also reflect the new value (bidirectional sync)
        .then(assert_world!("SliderControl Speed updated to 2.0", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Speed" && (ctrl.value - 2.0).abs() < 0.2)
        }))
        .then(assertions::log_summary("store_bidirectional summary"))
        .then(Action::Screenshot("store_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── save_load ─────────────────────────────────────────────────────────────────

fn save_load() -> Scenario {
    Scenario::builder("save_load")
        .description(
            "Modify Speed to 2.0, save to JSON, reset to defaults, load — verify the store and \
             slider reflect the saved value after the full cycle",
        )
        .then(Action::WaitFrames(90))
        // Initial state: Speed at default 5.0
        .then(assertions::resource_satisfies::<PaneStore>(
            "Speed at default 5.0 before save/load cycle",
            |store| {
                store
                    .try_get::<f64>("All Controls", "Speed")
                    .is_some_and(|v| (v - 5.0).abs() < 1e-4)
            },
        ))
        .then(Action::Screenshot("save_load_initial".into()))
        // e2e_save_load fires: set@100 → save@110 → reset@130 → load@150
        // We're at ~90 frames; wait 80 for the full cycle to complete
        .then(Action::WaitFrames(80))
        // After load, Speed should be 2.0 (the saved value, not the reset default)
        .then(assertions::resource_satisfies::<PaneStore>(
            "PaneStore Speed restored to 2.0 after load",
            |store| {
                store
                    .try_get::<f64>("All Controls", "Speed")
                    .is_some_and(|v| (v - 2.0).abs() < 1e-4)
            },
        ))
        // SliderControl should also reflect the loaded value
        .then(assert_world!("SliderControl Speed reflects loaded value 2.0", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Speed" && (ctrl.value - 2.0).abs() < 0.2)
        }))
        .then(assertions::log_summary("save_load summary"))
        .then(Action::Screenshot("save_load_restored".into()))
        .then(Action::WaitFrames(1))
        .build()
}

// ── reset_all ─────────────────────────────────────────────────────────────────

fn reset_all() -> Scenario {
    Scenario::builder("reset_all")
        .description(
            "Modify Speed and Volume sliders, trigger Reset All, verify both return to their \
             initial values",
        )
        .then(Action::WaitFrames(90))
        // Confirm controls start at default values
        .then(assert_world!("Speed starts at 5.0", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Speed" && (ctrl.value - 5.0).abs() < 1e-6)
        }))
        .then(assert_world!("Volume starts at 0.8", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Volume" && (ctrl.value - 0.8).abs() < 1e-6)
        }))
        .then(Action::Screenshot("reset_before".into()))
        // e2e_modify_then_reset: modifies at frame 100, resets at frame 130
        // We are at ~90 now; wait 60 frames for both operations to complete
        .then(Action::WaitFrames(60))
        // After reset, both sliders should be back at their initial values
        .then(assert_world!("Speed restored to 5.0 after reset", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Speed" && (ctrl.value - 5.0).abs() < 0.01)
        }))
        .then(assert_world!("Volume restored to 0.8 after reset", w => {
            let mut q = w.query::<(&PaneControlMeta, &SliderControl)>();
            q.iter(w).any(|(meta, ctrl)| meta.label == "Volume" && (ctrl.value - 0.8).abs() < 0.01)
        }))
        .then(assertions::log_summary("reset_all summary"))
        .then(Action::Screenshot("reset_after".into()))
        .then(Action::WaitFrames(1))
        .build()
}
