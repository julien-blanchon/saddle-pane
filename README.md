# saddle-pane

Lightweight debug & tweaking panel for [Bevy 0.18](https://bevyengine.org/) — drop-in sliders, toggles, color pickers and more, built entirely on Bevy's native UI.

Inspired by [Tweakpane](https://tweakpane.github.io/docs/), [Leva](https://github.com/pmndrs/leva), and [lil-gui](https://lil-gui.georgealways.com/). No egui dependency.

## Quick Start

Add `saddle-pane` to your project:

```toml
[dependencies]
saddle-pane = { git = "https://github.com/julien-blanchon/saddle-pane" }
```

Derive `Pane` on any `Resource` and register it:

```rust
use bevy::prelude::*;
use saddle_pane::prelude::*;

#[derive(Resource, Default, Pane)]
#[pane(title = "Settings")]
struct Settings {
    #[pane(slider, min = 0.0, max = 10.0, step = 0.1)]
    speed: f32,

    #[pane(slider, min = 0.0, max = 1.0)]
    volume: f32,

    enabled: bool, // auto-detected as toggle

    #[pane(color)]
    tint: Color,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanePlugin)
        .register_pane::<Settings>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .run();
}
```

That's it. `register_pane` initializes the resource, spawns the UI panel, and keeps both sides in sync automatically. Read your settings like any Bevy resource:

```rust
fn gameplay(settings: Res<Settings>) {
    let speed = settings.speed; // always in sync with the UI
}
```

## Features

- **Type-safe** — no string keys, no `Reflect`. The derive macro generates typed accessors at compile time
- **Bidirectional sync** — edit in the UI or mutate the resource in code, both sides stay in sync
- **Auto-detection** — `bool` → toggle, `f32` → number input, `String` → text, `Color` → color picker
- **All Bevy UI** — built on `bevy_flair` (CSS) + `bevy_ui_widgets`. No egui, no retained-mode
- **Non-intrusive** — game code needs zero `saddle_pane` imports. Add debug overlays from the outside

## Controls

| Type | Attribute | Field Type |
|------|-----------|------------|
| Slider | `#[pane(slider, min = 0.0, max = 10.0)]` | `f32`, `f64`, integers |
| Number | `#[pane(number)]` or auto | `f32`, `f64`, integers |
| Toggle | auto | `bool` |
| Text | auto | `String` |
| Color Picker | `#[pane(color)]` | `Color` |
| Dropdown | `#[pane(select(options = ["A", "B"]))]` | `usize` |
| Monitor | `#[pane(monitor)]` | any (read-only display) |
| Custom | `#[pane(custom = "control_id")]` | via plugin |

## Derive Attributes

### Struct-level

```rust
#[derive(Resource, Default, Pane)]
#[pane(title = "Physics")]              // panel title (defaults to struct name)
#[pane(position = "top-right")]         // top-left, top-right, bottom-left, bottom-right, or "x, y"
struct Physics { /* ... */ }
```

### Field-level

```rust
#[derive(Resource, Pane)]
#[pane(title = "Demo")]
struct Demo {
    // Slider with range and step
    #[pane(slider, min = 0.0, max = 100.0, step = 0.5)]
    speed: f32,

    // Number input with custom step
    #[pane(number, step = 5.0)]
    score: f32,

    // Color picker
    #[pane(color)]
    tint: Color,

    // Dropdown from fixed options
    #[pane(select(options = ["Low", "Medium", "High"]))]
    quality: usize,

    // Read-only monitor (game writes, UI displays)
    #[pane(monitor)]
    fps: f32,

    // Custom display label
    #[pane(label = "Player Name")]
    name: String,

    // Organize into collapsible folders
    #[pane(folder = "Advanced")]
    damping: f32,

    // Organize into tabs
    #[pane(tab = "Physics")]
    gravity: f32,

    // Hover tooltip
    #[pane(tooltip = "Downward acceleration in m/s²")]
    gravity_help: f32,

    // Macro-generated default value
    #[pane(default = 9.81)]
    gravity_default: f32,

    // Field ordering
    #[pane(order = 0)]
    first_field: f32,

    // Exclude from pane
    #[pane(skip)]
    _internal: u32,
}
```

## Patterns

### Editable Settings

Tweak values in real-time. Game code reads the resource directly:

```rust
#[derive(Resource, Pane)]
#[pane(title = "Physics")]
struct PhysicsConfig {
    #[pane(slider, min = -20.0, max = 0.0, default = -9.81)]
    gravity: f32,

    #[pane(slider, min = 0.0, max = 1.0, default = 0.3)]
    friction: f32,
}

fn physics_system(config: Res<PhysicsConfig>) {
    // Values update in real-time as you drag sliders
    apply_gravity(config.gravity);
}
```

### Monitor-Only Stats

Display game state without editing. Write to the resource, the UI updates automatically:

```rust
#[derive(Resource, Default, Pane)]
#[pane(title = "Stats")]
struct GameStats {
    #[pane(monitor)]
    fps: f32,

    #[pane(monitor, label = "Entity Count")]
    entities: u32,
}

fn update_stats(mut stats: ResMut<GameStats>, /* ... */) {
    stats.fps = 60.0;
    stats.entities = 1234;
}
```

### External Debug Overlay

Add debug panels to existing game code **without modifying it**:

```rust
// Game module — zero pane imports
mod game {
    #[derive(Resource)]
    pub struct SimConfig {
        pub gravity: f32,
        pub bounciness: f32,
    }
}

// Debug module — bridges game state ↔ pane
mod debug {
    use saddle_pane::prelude::*;

    #[derive(Resource, Pane)]
    #[pane(title = "Sim Config")]
    pub struct SimPane {
        #[pane(slider, min = -800.0, max = 0.0, default = -400.0)]
        gravity: f32,

        #[pane(slider, min = 0.0, max = 1.0, default = 0.8)]
        bounciness: f32,
    }

    fn sync(pane: Res<SimPane>, mut config: ResMut<super::game::SimConfig>) {
        if pane.is_changed() && !pane.is_added() {
            config.gravity = pane.gravity;
            config.bounciness = pane.bounciness;
        }
    }
}
```

### Multiple Panes

Register as many panes as you need — each gets its own UI panel:

```rust
app.register_pane::<PhysicsConfig>()
   .register_pane::<VisualsConfig>()
   .register_pane::<GameStats>();
```

## Events

React to value changes and button presses via Bevy observers:

```rust
app.add_observer(|ev: On<PaneChanged>| {
    info!("{}/{} = {:?}", ev.event().pane, ev.event().field, ev.event().value);
});

app.add_observer(|ev: On<PaneButtonPressed>| {
    if ev.event().label == "Reset" {
        // handle reset
    }
});
```

## Theming

Toggle between dark and light themes:

```rust
fn toggle_theme(mut theme: ResMut<PaneTheme>) {
    theme.toggle(); // Dark ↔ Light
}

// Or override per-pane:
commands.entity(pane).insert(PaneThemeOverride(PaneThemePreset::Light));
```

## Persistence

Save and restore all pane values as JSON (requires `serialize` feature):

```toml
saddle-pane = { git = "...", features = ["serialize"] }
```

```rust
let json = store.save_json();
store.load_json(&json)?;
```

## Plugin Controls

Extend with custom control types via plugin crates:

| Plugin | Control | Description |
|--------|---------|-------------|
| `saddle-pane-interval` | Range slider | Dual-thumb min..max range |
| `saddle-pane-vector2` | 2D joystick | Joystick pad + X/Y fields |
| `saddle-pane-button-grid` | Button grid | Grid of clickable buttons |
| `saddle-pane-radio-grid` | Radio grid | Single/multi-select grid |
| `saddle-pane-bezier` | Bezier editor | Cubic bezier curve with presets |
| `saddle-pane-file` | File browser | Native file dialog + path display |

## Examples

Run examples from the `examples/` directory:

```bash
cd examples
cargo run -p saddle-pane-example-basic           # Minimal derive setup
cargo run -p saddle-pane-example-bouncing-balls   # Bouncing balls with debug overlay
cargo run -p saddle-pane-example-game-debug       # Three debug patterns
cargo run -p saddle-pane-lab                      # Kitchen sink with all controls
```

## Compatibility

| saddle-pane | Bevy |
|-------------|------|
| 0.1         | 0.18 |

## License

MIT-0
