use std::any::Any;
use std::collections::HashMap;

use bevy::prelude::*;

use crate::controls::PaneControlMeta;
use crate::controls::PaneValue;

// ── ControlConfig: type-erased configuration bag ──

/// Configuration values for custom controls.
#[derive(Clone, Debug)]
pub enum ConfigValue {
    Float(f64),
    Bool(bool),
    String(String),
    Color(Color),
    Int(i64),
    FloatPair(f64, f64),
    FloatList(Vec<f64>),
    StringList(Vec<String>),
}

/// Type-erased configuration bag produced by builder methods,
/// consumed by a control's spawn function.
#[derive(Clone, Debug, Default)]
pub struct ControlConfig {
    pub params: HashMap<String, ConfigValue>,
}

impl ControlConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, key: &str, value: ConfigValue) -> Self {
        self.params.insert(key.to_string(), value);
        self
    }

    pub fn float(self, key: &str, value: f64) -> Self {
        self.set(key, ConfigValue::Float(value))
    }

    pub fn bool(self, key: &str, value: bool) -> Self {
        self.set(key, ConfigValue::Bool(value))
    }

    pub fn string(self, key: &str, value: &str) -> Self {
        self.set(key, ConfigValue::String(value.to_string()))
    }

    pub fn int(self, key: &str, value: i64) -> Self {
        self.set(key, ConfigValue::Int(value))
    }

    pub fn float_pair(self, key: &str, a: f64, b: f64) -> Self {
        self.set(key, ConfigValue::FloatPair(a, b))
    }

    pub fn float_list(self, key: &str, values: Vec<f64>) -> Self {
        self.set(key, ConfigValue::FloatList(values))
    }

    pub fn string_list(self, key: &str, values: Vec<String>) -> Self {
        self.set(key, ConfigValue::StringList(values))
    }

    // ── Typed getters ──

    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.params.get(key)? {
            ConfigValue::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.params.get(key)? {
            ConfigValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.params.get(key)? {
            ConfigValue::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.params.get(key)? {
            ConfigValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn get_float_pair(&self, key: &str) -> Option<(f64, f64)> {
        match self.params.get(key)? {
            ConfigValue::FloatPair(a, b) => Some((*a, *b)),
            _ => None,
        }
    }

    pub fn get_float_list(&self, key: &str) -> Option<&[f64]> {
        match self.params.get(key)? {
            ConfigValue::FloatList(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_string_list(&self, key: &str) -> Option<&[String]> {
        match self.params.get(key)? {
            ConfigValue::StringList(v) => Some(v),
            _ => None,
        }
    }
}

// ── PaneCustomValue: trait-object for plugin values ──

/// Trait for custom values stored in `PaneValue::Custom`.
/// Must be object-safe, cloneable, and comparable.
pub trait PaneCustomValue: Send + Sync + std::fmt::Debug + 'static {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn PaneCustomValue>;
    fn eq_box(&self, other: &dyn PaneCustomValue) -> bool;
}

/// Wrapper around `Box<dyn PaneCustomValue>` that implements Clone, Debug, PartialEq.
#[derive(Clone)]
pub struct CustomValueBox(pub Box<dyn PaneCustomValue>);

impl Clone for Box<dyn PaneCustomValue> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for CustomValueBox {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_box(other.0.as_ref())
    }
}

impl std::fmt::Debug for CustomValueBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── PaneControlPlugin descriptor ──

/// Spawn function signature for custom controls.
pub type SpawnFn = fn(
    parent: &mut ChildSpawnerCommands,
    meta: &PaneControlMeta,
    config: &ControlConfig,
    asset_server: &AssetServer,
) -> Entity;

/// Default-value function signature.
pub type DefaultValueFn = fn(config: &ControlConfig) -> Option<PaneValue>;

/// Descriptor for a custom pane control plugin.
/// External crates create these and register them in `PaneControlRegistry`.
pub struct PaneControlPlugin {
    /// Unique identifier (e.g., "interval", "bezier").
    pub id: &'static str,
    /// Build function: registers assets, systems, and observers with the app.
    /// Called once during plugin registration.
    pub build: fn(&mut App),
    /// Spawn function: creates the control's UI entity hierarchy.
    pub spawn: SpawnFn,
    /// Convert config to initial PaneValue for the store (None for non-value controls).
    pub default_value: DefaultValueFn,
}

/// Resource: registry of all custom control plugins ("the plugin pool").
#[derive(Resource, Default)]
pub struct PaneControlRegistry {
    plugins: Vec<PaneControlPlugin>,
}

impl PaneControlRegistry {
    /// Register a custom control plugin. Later registrations have higher priority.
    pub fn register(&mut self, plugin: PaneControlPlugin) {
        self.plugins.insert(0, plugin);
    }

    /// Look up a control plugin by ID.
    pub fn get(&self, id: &str) -> Option<&PaneControlPlugin> {
        self.plugins.iter().find(|p| p.id == id)
    }

    /// Snapshot all spawn functions as a map (for use during pane spawning).
    pub fn spawn_fns(&self) -> std::collections::HashMap<String, SpawnFn> {
        self.plugins
            .iter()
            .map(|p| (p.id.to_string(), p.spawn))
            .collect()
    }

    /// Collect all build functions for registered plugins.
    pub fn build_fns(&self) -> Vec<fn(&mut bevy::prelude::App)> {
        self.plugins.iter().map(|p| p.build).collect()
    }

    /// Snapshot all default-value functions as a map.
    pub fn default_fns(&self) -> std::collections::HashMap<String, DefaultValueFn> {
        self.plugins
            .iter()
            .map(|p| (p.id.to_string(), p.default_value))
            .collect()
    }
}
