use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::controls::PaneValue;

type StoreKey = (String, String);

fn make_key(pane: &str, field: &str) -> StoreKey {
    (pane.to_string(), field.to_string())
}

/// Central value store for builder-API panes.
///
/// Values are keyed by `(pane_title, control_label)`.
/// Controls sync their values here, and user code reads from here.
#[derive(Resource, Default, Debug)]
pub struct PaneStore {
    values: HashMap<StoreKey, PaneValue>,
    initial_values: HashMap<StoreKey, PaneValue>,
    /// Keys that were set externally (via `set`/`set_raw`) and need syncing to controls.
    dirty: HashSet<StoreKey>,
}

impl PaneStore {
    /// Get a typed value from the store.
    ///
    /// # Panics
    /// Panics if the key doesn't exist or the type doesn't match.
    pub fn get<T: FromPaneValue>(&self, pane: &str, field: &str) -> T {
        let key = make_key(pane, field);
        let value = self
            .values
            .get(&key)
            .unwrap_or_else(|| panic!("PaneStore: no value for ({pane:?}, {field:?})"));
        T::from_pane_value(value)
    }

    /// Try to get a typed value, returning `None` if the key doesn't exist or the type doesn't match.
    pub fn try_get<T: FromPaneValue>(&self, pane: &str, field: &str) -> Option<T> {
        let key = make_key(pane, field);
        self.values.get(&key).and_then(T::try_from_pane_value)
    }

    /// Get a typed value with a fallback default if the key doesn't exist.
    pub fn get_or<T: FromPaneValue>(&self, pane: &str, field: &str, default: T) -> T {
        self.try_get(pane, field).unwrap_or(default)
    }

    /// Get the raw `PaneValue`, or `None` if not present.
    pub fn get_raw(&self, pane: &str, field: &str) -> Option<&PaneValue> {
        let key = make_key(pane, field);
        self.values.get(&key)
    }

    /// Check if a key exists in the store.
    pub fn contains(&self, pane: &str, field: &str) -> bool {
        let key = make_key(pane, field);
        self.values.contains_key(&key)
    }

    /// Set a value in the store from external code.
    /// Marks the key as dirty so the UI will be updated to match.
    pub fn set(&mut self, pane: &str, field: &str, value: impl IntoPaneValue) {
        let key = make_key(pane, field);
        let pane_value = value.into_pane_value();
        if self.values.get(&key) != Some(&pane_value) {
            self.dirty.insert(key.clone());
            self.values.insert(key, pane_value);
        }
    }

    /// Set a raw PaneValue directly (for plugin controls).
    /// Marks the key as dirty so the UI will be updated to match.
    pub fn set_raw(&mut self, pane: &str, field: &str, value: PaneValue) {
        let key = make_key(pane, field);
        if self.values.get(&key) != Some(&value) {
            self.dirty.insert(key.clone());
            self.values.insert(key, value);
        }
    }

    /// Internal: set a value from UI sync (does NOT mark dirty — avoids feedback loops).
    pub(crate) fn set_from_ui(&mut self, pane: &str, field: &str, value: PaneValue) {
        let key = make_key(pane, field);
        self.values.insert(key, value);
    }

    /// Initialize a value (called during pane spawning).
    pub(crate) fn init(&mut self, pane: &str, field: &str, value: PaneValue) {
        let key = make_key(pane, field);
        self.initial_values.insert(key.clone(), value.clone());
        self.values.insert(key, value);
    }

    /// Reset a field to its initial value.
    /// Marks the key as dirty so the UI will be updated to match.
    pub fn reset(&mut self, pane: &str, field: &str) {
        let key = make_key(pane, field);
        if let Some(initial) = self.initial_values.get(&key).cloned() {
            self.dirty.insert(key.clone());
            self.values.insert(key, initial);
        }
    }

    /// Drain all dirty keys. Called by the store→control sync system.
    pub(crate) fn drain_dirty(&mut self) -> HashSet<StoreKey> {
        std::mem::take(&mut self.dirty)
    }

    /// Check if there are any dirty keys pending sync.
    pub(crate) fn has_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }
}

/// Convert from `PaneValue` to a concrete type.
pub trait FromPaneValue: Sized {
    fn from_pane_value(value: &PaneValue) -> Self;

    /// Try to convert, returning `None` on type mismatch.
    fn try_from_pane_value(value: &PaneValue) -> Option<Self>;
}

/// Convert a concrete type into `PaneValue`.
pub trait IntoPaneValue {
    fn into_pane_value(self) -> PaneValue;
}

impl FromPaneValue for f64 {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::Float(v) => *v,
            PaneValue::Int(v) => *v as f64,
            other => panic!("Expected Float, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::Float(v) => Some(*v),
            PaneValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }
}

impl FromPaneValue for f32 {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::Float(v) => *v as f32,
            PaneValue::Int(v) => *v as f32,
            other => panic!("Expected Float, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::Float(v) => Some(*v as f32),
            PaneValue::Int(v) => Some(*v as f32),
            _ => None,
        }
    }
}

impl FromPaneValue for bool {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::Bool(v) => *v,
            other => panic!("Expected Bool, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

impl FromPaneValue for String {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::String(v) => v.clone(),
            other => panic!("Expected String, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::String(v) => Some(v.clone()),
            _ => None,
        }
    }
}

impl FromPaneValue for i64 {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::Int(v) => *v,
            PaneValue::Float(v) => *v as i64,
            other => panic!("Expected Int, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::Int(v) => Some(*v),
            PaneValue::Float(v) => Some(*v as i64),
            _ => None,
        }
    }
}

impl FromPaneValue for usize {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::Int(v) => *v as usize,
            PaneValue::Float(v) => *v as usize,
            other => panic!("Expected Int, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::Int(v) => Some(*v as usize),
            PaneValue::Float(v) => Some(*v as usize),
            _ => None,
        }
    }
}

impl FromPaneValue for Color {
    fn from_pane_value(value: &PaneValue) -> Self {
        match value {
            PaneValue::Color(v) => *v,
            other => panic!("Expected Color, got {other:?}"),
        }
    }
    fn try_from_pane_value(value: &PaneValue) -> Option<Self> {
        match value {
            PaneValue::Color(v) => Some(*v),
            _ => None,
        }
    }
}

impl IntoPaneValue for f64 {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::Float(self)
    }
}

impl IntoPaneValue for f32 {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::Float(self as f64)
    }
}

impl IntoPaneValue for bool {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::Bool(self)
    }
}

impl IntoPaneValue for String {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::String(self)
    }
}

impl IntoPaneValue for &str {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::String(self.to_string())
    }
}

impl IntoPaneValue for i64 {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::Int(self)
    }
}

impl IntoPaneValue for usize {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::Int(self as i64)
    }
}

impl IntoPaneValue for Color {
    fn into_pane_value(self) -> PaneValue {
        PaneValue::Color(self)
    }
}

// ── Serialization (behind "serialize" feature) ──

#[cfg(feature = "serialize")]
mod persistence {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// Serializable subset of `PaneValue` (excludes `Custom`).
    #[derive(Serialize, Deserialize, Clone, Debug)]
    enum SerializableValue {
        Float(f64),
        Bool(bool),
        String(String),
        Color([f32; 4]),
        Int(i64),
    }

    impl SerializableValue {
        fn from_pane_value(v: &PaneValue) -> Option<Self> {
            match v {
                PaneValue::Float(f) => Some(Self::Float(*f)),
                PaneValue::Bool(b) => Some(Self::Bool(*b)),
                PaneValue::String(s) => Some(Self::String(s.clone())),
                PaneValue::Color(c) => {
                    let srgba = c.to_srgba();
                    Some(Self::Color([srgba.red, srgba.green, srgba.blue, srgba.alpha]))
                }
                PaneValue::Int(i) => Some(Self::Int(*i)),
                PaneValue::Custom(_) => None,
            }
        }

        fn to_pane_value(&self) -> PaneValue {
            match self {
                Self::Float(f) => PaneValue::Float(*f),
                Self::Bool(b) => PaneValue::Bool(*b),
                Self::String(s) => PaneValue::String(s.clone()),
                Self::Color([r, g, b, a]) => {
                    PaneValue::Color(Color::srgba(*r, *g, *b, *a))
                }
                Self::Int(i) => PaneValue::Int(*i),
            }
        }
    }

    /// Serializable snapshot of the pane store.
    #[derive(Serialize, Deserialize)]
    struct StoreSnapshot {
        values: Vec<(StoreKey, SerializableValue)>,
    }

    impl PaneStore {
        /// Serialize all non-Custom values to a JSON string.
        pub fn save_json(&self) -> String {
            let values: Vec<_> = self
                .values
                .iter()
                .filter_map(|(k, v)| {
                    SerializableValue::from_pane_value(v).map(|sv| (k.clone(), sv))
                })
                .collect();
            let snapshot = StoreSnapshot { values };
            serde_json::to_string_pretty(&snapshot).unwrap_or_default()
        }

        /// Load values from a JSON string, overwriting matching keys.
        /// Loaded keys are marked dirty so the UI updates to match.
        pub fn load_json(&mut self, json: &str) -> Result<usize, String> {
            let snapshot: StoreSnapshot =
                serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;
            let mut count = 0;
            for (key, sv) in snapshot.values {
                let pane_value = sv.to_pane_value();
                self.dirty.insert(key.clone());
                self.values.insert(key, pane_value);
                count += 1;
            }
            Ok(count)
        }
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
