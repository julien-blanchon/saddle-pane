use saddle_bevy_e2e::action::Action;
use saddle_bevy_e2e::scenario::Scenario;

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
        "theme_light" => Some(theme_light()),
        "store_bidirectional" => Some(store_bidirectional()),
        "save_load" => Some(save_load()),
        _ => None,
    }
}

fn smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot and verify pane renders")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("smoke_launch".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn all_controls_visible() -> Scenario {
    Scenario::builder("all_controls_visible")
        .description("Screenshot showing all control types")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("all_controls_01".into()))
        .then(Action::WaitFrames(60))
        .then(Action::Screenshot("all_controls_02".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn dropdown_open() -> Scenario {
    Scenario::builder("dropdown_open")
        .description("Open the Quality dropdown programmatically and screenshot")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("dropdown_before".into()))
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("dropdown_open".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn color_picker_open() -> Scenario {
    Scenario::builder("color_picker_open")
        .description("Open the color picker programmatically and screenshot")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("color_picker_before".into()))
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("color_picker_open".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn tab_screenshot(name: &str) -> Scenario {
    Scenario::builder(name)
        .description(format!("Screenshot the {name} tab"))
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot(format!("{name}_01")))
        // Wait for live-updating read-only controls to show values
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot(format!("{name}_02")))
        .then(Action::WaitFrames(10))
        .build()
}

fn theme_light() -> Scenario {
    Scenario::builder("theme_light")
        .description("Switch to light theme and screenshot")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("theme_dark_before".into()))
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("theme_light_after".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn save_load() -> Scenario {
    Scenario::builder("save_load")
        .description("Modify Speed, save, reset, load, verify restored")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("save_load_initial".into()))
        .then(Action::WaitFrames(70))
        .then(Action::Screenshot("save_load_restored".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn store_bidirectional() -> Scenario {
    Scenario::builder("store_bidirectional")
        .description("Set Speed via store.set() and verify slider updates")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("store_before".into()))
        .then(Action::WaitFrames(30))
        .then(Action::Screenshot("store_after".into()))
        .then(Action::WaitFrames(10))
        .build()
}

fn reset_all() -> Scenario {
    Scenario::builder("reset_all")
        .description("Modify controls, then reset and screenshot")
        .then(Action::WaitFrames(90))
        .then(Action::Screenshot("reset_before".into()))
        .then(Action::WaitFrames(60))
        .then(Action::Screenshot("reset_after".into()))
        .then(Action::WaitFrames(10))
        .build()
}
