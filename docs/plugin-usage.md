# Using saddle-pane Plugins

This guide explains how to install and use external saddle_pane control plugins.

## Installing a Plugin

### 1. Add the dependency

```toml
# Cargo.toml
[dependencies]
saddle_pane = "0.1"
saddle_pane_interval = "0.1"  # the plugin crate
```

### 2. Register the plugin

Add the plugin **after** `PanePlugin`:

```rust
use bevy::prelude::*;
use saddle_pane::prelude::*;
use saddle_pane_interval::PaneIntervalPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // ... other plugins (FlairPlugin, InputDispatchPlugin, UiWidgetsPlugins)
        .add_plugins(PanePlugin)
        .add_plugins(PaneIntervalPlugin)  // after PanePlugin
        .add_systems(Startup, setup)
        .run();
}
```

### 3. Use the control

```rust
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    PaneBuilder::new("Debug")
        .slider("Speed", 0.0..=10.0, 5.0)
        // Use the interval control (built-in convenience method)
        .interval("Range", 0.0..=100.0, 20.0..=80.0)
        .spawn(&mut commands);
}
```

## Using `custom()` Directly

If a plugin doesn't provide a convenience builder method, use `custom()`:

```rust
use saddle_pane::prelude::ControlConfig;

PaneBuilder::new("Debug")
    .custom("interval", "Range", ControlConfig::new()
        .float("bounds_min", 0.0)
        .float("bounds_max", 100.0)
        .float("default_min", 20.0)
        .float("default_max", 80.0)
        .float("step", 0.5))
    .spawn(&mut commands);
```

## Reading Plugin Values

Plugin controls sync their values to `PaneStore` like built-in controls. For custom value types, use `PaneValue::Custom` and downcast:

```rust
use saddle_pane_interval::IntervalValue;

fn read_interval(store: Res<PaneStore>) {
    if let Some(PaneValue::Custom(boxed)) = store.get_raw("Debug", "Range") {
        if let Some(interval) = boxed.0.as_any().downcast_ref::<IntervalValue>() {
            println!("Range: {}..{}", interval.min, interval.max);
        }
    }
}
```

Or observe changes:

```rust
fn on_change(ev: On<PaneChanged>) {
    if let PaneValue::Custom(ref boxed) = ev.event().value {
        if let Some(interval) = boxed.0.as_any().downcast_ref::<IntervalValue>() {
            info!("{}: {}..{}", ev.event().field, interval.min, interval.max);
        }
    }
}
```

## Available Plugins

| Plugin | Crate | Control ID | Description |
|--------|-------|------------|-------------|
| Interval | `saddle_pane_interval` | `"interval"` | Dual-thumb range slider for selecting a min..max sub-range |

## Plugin Ordering

Plugins must be added **after** `PanePlugin` because they need the `PaneControlRegistry` resource to exist. The order between plugins doesn't matter.

```rust
app.add_plugins(PanePlugin);              // must be first
app.add_plugins(PaneIntervalPlugin);      // any order after PanePlugin
app.add_plugins(PaneOtherPlugin);         // any order after PanePlugin
```
