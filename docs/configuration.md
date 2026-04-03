# Configuration

## PanePlugin

Add `PanePlugin` to your app. It registers all core systems and embeds CSS assets.

```rust
app.add_plugins(PanePlugin);
```

No configuration parameters — it just works.

## Derive Attributes

### Struct-level

| Attribute | Type | Default | Effect |
|-----------|------|---------|--------|
| `title` | `&str` | struct name | Panel title displayed in header |
| `position` | `&str` | auto | Panel position: `"top-left"`, `"top-right"`, `"bottom-left"`, `"bottom-right"`, or `"x, y"` |

### Field-level

| Attribute | Type | Default | Effect |
|-----------|------|---------|--------|
| `slider` | flag | — | Use slider control |
| `number` | flag | — | Use number input control |
| `color` | flag | — | Use color picker control |
| `monitor` | flag | — | Read-only display (game writes, UI shows) |
| `select(options = [...])` | string list | — | Dropdown with fixed options |
| `custom = "id"` | `&str` | — | Use a plugin-registered control |
| `min` | number | — | Slider/number minimum value |
| `max` | number | — | Slider/number maximum value |
| `step` | number | — | Slider/number step increment |
| `default` | value | — | Default value (generates Default impl) |
| `label` | `&str` | field name | Override display label |
| `folder` | `&str` | — | Place in collapsible folder |
| `tab` | `&str` | — | Place in tab page |
| `tooltip` | `&str` | — | Hover help text |
| `icon` | `&str` | — | SVG string for label icon |
| `order` | integer | — | Display order within tab/folder |
| `skip` | flag | — | Exclude field from pane |

### Auto-detection

When no explicit control attribute is given, the field type determines the control:

| Field Type | Control |
|------------|---------|
| `bool` | Toggle (checkbox) |
| `f32`, `f64` | Number input |
| `i32`, `u32`, `i64`, `u64`, `usize` | Number input |
| `String` | Text input |
| `Color` | Color picker |

## PaneTheme

Global theme resource. Default: dark.

```rust
// Toggle dark ↔ light
theme.toggle();

// Set explicitly
theme.preset = PaneThemePreset::Light;
```

### Per-pane override

```rust
commands.entity(pane_entity).insert(PaneThemeOverride(PaneThemePreset::Light));
```

## PaneStore

Central value store. Access via `Res<PaneStore>`.

| Method | Description |
|--------|-------------|
| `store.get::<T>(pane, field)` | Get typed value (panics if missing) |
| `store.try_get::<T>(pane, field)` | Get typed value (returns Option) |
| `store.set(pane, field, value)` | Set value from code |
| `store.read(&handle)` | Read via typed PaneHandle |
| `store.save_json()` | Serialize all values (needs `serialize` feature) |
| `store.load_json(&json)` | Deserialize values (needs `serialize` feature) |
| `store.reset(pane, field)` | Reset to initial value |
| `store.reset_all()` | Reset all values |

## Events

| Event | Trigger | Fields |
|-------|---------|--------|
| `PaneChanged` | Any value change | `pane`, `field`, `value` |
| `PaneButtonPressed` | Button click | `pane`, `label` |
| `PaneEditStart` | Text/number field focused | `pane`, `field` |
| `PaneEditEnd` | Text/number field unfocused | `pane`, `field` |

All events are observer-based (use `app.add_observer()`).

## Features

| Feature | Default | Effect |
|---------|---------|--------|
| `derive` | yes | Enables `#[derive(Pane)]` macro |
| `serialize` | no | Enables JSON save/load via `PaneStore` |
