# saddle-pane Plugin Development Guide

This guide explains how to create a custom control plugin for saddle_pane.

## Overview

saddle_pane has a plugin system inspired by [Tweakpane plugins](https://tweakpane.github.io/docs/plugins/dev/). Each plugin registers a **control type** that can be used in any pane via `PaneBuilder::custom()`.

A plugin provides four things:
1. **Spawn function** — builds the UI entity hierarchy for the control
2. **Build function** — registers systems and observers with the Bevy app
3. **Default value function** — returns the initial `PaneValue` for the store
4. **CSS file** — embedded stylesheet for the control's appearance

## Crate Structure

```
saddle_pane_my_control/
├── Cargo.toml
└── src/
    ├── lib.rs            # Plugin struct, component, systems, spawn
    └── style/
        └── my_control.css  # Embedded CSS
```

### Cargo.toml

```toml
[package]
name = "saddle_pane_my_control"
version = "0.1.0"
edition = "2024"

[dependencies]
bevy = "0.18"
bevy_flair = "0.7"
bevy_ui_widgets = "0.18"
saddle_pane = { path = "../saddle_pane" }  # or version from registry
```

## Step-by-Step Implementation

### 1. Define Your Value Type

If your control produces a custom value (not just `f64`, `bool`, `String`, `Color`), implement `PaneCustomValue`:

```rust
use saddle_pane::prelude::PaneCustomValue;

#[derive(Clone, Debug, PartialEq)]
pub struct MyValue {
    pub x: f64,
    pub y: f64,
}

impl PaneCustomValue for MyValue {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_box(&self) -> Box<dyn PaneCustomValue> { Box::new(self.clone()) }
    fn eq_box(&self, other: &dyn PaneCustomValue) -> bool {
        other.as_any().downcast_ref::<MyValue>().is_some_and(|o| o == self)
    }
}
```

### 2. Define Your Component

The component lives on the control's row entity and is the source of truth for the control's value:

```rust
#[derive(Component, Clone, Debug)]
pub struct MyControl {
    pub x: f64,
    pub y: f64,
    // ... config fields
}
```

### 3. Write the Spawn Function

The spawn function builds the UI hierarchy. It receives a `ControlConfig` bag with parameters from the builder.

Key utilities from saddle_pane you can use:
- `saddle_pane::controls::spawn_label(parent, &label)` — creates the label column
- `saddle_pane::controls::pane_font(size)` — standard font
- `saddle_pane::controls::css_percent(value)` — format `"42.0000%"`

```rust
use saddle_pane::controls::{PaneControlMeta, spawn_label, pane_font, css_percent};
use saddle_pane::registry::ControlConfig;

fn spawn_my_control(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity {
    let default_x = config.get_float("x").unwrap_or(0.0);
    // ... read config

    let mut row_entity = Entity::PLACEHOLDER;

    parent.spawn((
        Node::default(),
        ClassList::new("pane-row"),
        NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
        meta.clone(),
        MyControl { x: default_x, /* ... */ },
    )).with_children(|row| {
        row_entity = row.target_entity();
        spawn_label(row, &meta.label);
        // ... build your UI
    });

    row_entity
}
```

### 4. Write Systems

Your plugin needs up to three kinds of systems, registered in saddle_pane's system sets:

```rust
use saddle_pane::prelude::PaneSystems;

// Display: update UI when component changes
fn update_my_display(
    q: Query<(Entity, &MyControl), Changed<MyControl>>,
    // ... queries for UI elements
) {
    // Update text, styles, etc.
}

// Sync: write to PaneStore + trigger PaneChanged
fn sync_my_to_store(
    mut store: ResMut<PaneStore>,
    mut commands: Commands,
    q: Query<(&PaneControlMeta, &MyControl), Changed<MyControl>>,
) {
    for (meta, ctrl) in &q {
        let value = PaneValue::Custom(CustomValueBox(Box::new(MyValue { x: ctrl.x, y: ctrl.y })));
        if store.get_raw(&meta.pane_title, &meta.label) != Some(&value) {
            store.set_raw(&meta.pane_title, &meta.label, value.clone());
            commands.trigger(PaneChanged {
                pane: meta.pane_title.clone(),
                field: meta.label.clone(),
                value,
            });
        }
    }
}
```

### 5. Write the Plugin

```rust
use bevy::asset::embedded_asset;

const STYLE_PATH: &str = "embedded://saddle_pane_my_control/style/my_control.css";

pub struct PaneMyControlPlugin;

impl Plugin for PaneMyControlPlugin {
    fn build(&self, app: &mut App) {
        // Embed CSS
        embedded_asset!(app, "style/my_control.css");

        // Register in the pane control registry
        let mut registry = app.world_mut().resource_mut::<PaneControlRegistry>();
        registry.register(PaneControlPlugin {
            id: "my_control",
            build: build_systems,
            spawn: spawn_my_control,
            default_value: default_value,
        });

        // Register systems
        build_systems(app);
    }
}

fn build_systems(app: &mut App) {
    app.add_systems(PostUpdate, update_my_display.in_set(PaneSystems::Display));
    app.add_systems(PostUpdate, sync_my_to_store.in_set(PaneSystems::Sync));
}

fn default_value(config: &ControlConfig) -> Option<PaneValue> {
    let x = config.get_float("x").unwrap_or(0.0);
    let y = config.get_float("y").unwrap_or(0.0);
    Some(PaneValue::Custom(CustomValueBox(Box::new(MyValue { x, y }))))
}
```

### 6. CSS

Create `src/style/my_control.css`. Each control row gets its own `NodeStyleSheet`, so your CSS is scoped:

```css
.pane-row {
    display: flex;
    height: 28px;
    min-height: 28px;
    min-width: 0px;
    align-items: center;
    padding: 0px 8px;
    column-gap: 6px;
    overflow: clip;
}

.my-control-widget {
    /* your styles */
}

.pane-label-text {
    color: #78797f;
    font-size: 11px;
    min-width: 0px;
    overflow: clip;
}
```

## System Set Reference

| Set | Schedule | Purpose |
|-----|----------|---------|
| `PaneSystems::Interaction` | PostUpdate | Widget events -> control component |
| `PaneSystems::Sync` | PostUpdate | Control component -> PaneStore + events |
| `PaneSystems::Display` | PostUpdate | Control component -> UI updates |

These run in order: Interaction -> Sync -> Display.

## ControlConfig Parameters

`ControlConfig` is a string-keyed bag of typed values. Use it to pass configuration from the builder to your spawn function:

```rust
// In user code (builder):
pane.custom("my_control", "Label", ControlConfig::new()
    .float("x", 1.0)
    .float("y", 2.0)
    .bool("locked", false)
    .string("mode", "default"))

// In your spawn function:
let x = config.get_float("x").unwrap_or(0.0);
let locked = config.get_bool("locked").unwrap_or(false);
```

Available types: `float`, `bool`, `string`, `int`, `float_pair`.

## Builder Extension Trait

You can add convenience methods to `PaneBuilder` via extension traits:

```rust
pub trait MyControlPaneExt {
    fn my_control(self, label: &str, x: f64, y: f64) -> Self;
}

impl MyControlPaneExt for saddle_pane::prelude::PaneBuilder {
    fn my_control(self, label: &str, x: f64, y: f64) -> Self {
        self.custom("my_control", label, ControlConfig::new()
            .float("x", x)
            .float("y", y))
    }
}
```

## Example: saddle_pane_interval

See `crates/saddle_pane_interval/` for a complete working example of an external plugin that implements a dual-thumb range slider.
