//! Typed handle for type-safe PaneStore access.

use std::marker::PhantomData;

use crate::controls::PaneValue;
use crate::store::{FromPaneValue, IntoPaneValue};

/// A typed key for accessing a value in [`PaneStore`](crate::store::PaneStore).
///
/// Provides compile-time type safety — you can't accidentally read a `f64`
/// handle as a `bool`.
///
/// ```rust,ignore
/// let speed = PaneHandle::<f64>::new("Debug", "Speed");
///
/// // Type-safe read:
/// let v: f64 = store.read(&speed);
///
/// // Type-safe write:
/// store.write(&speed, 7.5);
/// ```
#[derive(Debug)]
pub struct PaneHandle<T> {
    pane: String,
    field: String,
    _marker: PhantomData<T>,
}

impl<T> PaneHandle<T> {
    /// Create a handle for the given pane and field.
    pub fn new(pane: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            pane: pane.into(),
            field: field.into(),
            _marker: PhantomData,
        }
    }

    /// The pane title this handle points to.
    pub fn pane(&self) -> &str {
        &self.pane
    }

    /// The field label this handle points to.
    pub fn field(&self) -> &str {
        &self.field
    }
}

// Manual Clone/Copy since PhantomData doesn't constrain T
impl<T> Clone for PaneHandle<T> {
    fn clone(&self) -> Self {
        Self {
            pane: self.pane.clone(),
            field: self.field.clone(),
            _marker: PhantomData,
        }
    }
}

// ── PaneStore integration ──

use crate::store::PaneStore;

impl PaneStore {
    /// Read a typed value via handle. Panics if missing.
    pub fn read<T: FromPaneValue>(&self, handle: &PaneHandle<T>) -> T {
        self.get(&handle.pane, &handle.field)
    }

    /// Try to read a typed value via handle. Returns None if missing or type mismatch.
    pub fn try_read<T: FromPaneValue>(&self, handle: &PaneHandle<T>) -> Option<T> {
        self.try_get(&handle.pane, &handle.field)
    }

    /// Write a value via handle. UI updates automatically.
    pub fn write<T: IntoPaneValue>(&mut self, handle: &PaneHandle<T>, value: T) {
        self.set(&handle.pane, &handle.field, value);
    }

    /// Get the raw PaneValue via handle.
    pub fn read_raw<T>(&self, handle: &PaneHandle<T>) -> Option<&PaneValue> {
        self.get_raw(&handle.pane, &handle.field)
    }
}
