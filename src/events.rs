use bevy::prelude::*;

use crate::controls::PaneValue;

/// Fired whenever a pane control's value changes.
#[derive(Event, Clone, Debug)]
pub struct PaneChanged {
    pub pane: String,
    pub field: String,
    pub value: PaneValue,
}

/// Fired when a pane button is clicked.
#[derive(Event, Clone, Debug)]
pub struct PaneButtonPressed {
    pub pane: String,
    pub label: String,
}

/// Fired when inline editing of a control begins (user focuses a text/number field).
#[derive(Event, Clone, Debug)]
pub struct PaneEditStart {
    pub pane: String,
    pub field: String,
}

/// Fired when inline editing of a control ends (user blurs the field).
#[derive(Event, Clone, Debug)]
pub struct PaneEditEnd {
    pub pane: String,
    pub field: String,
    pub value: PaneValue,
    /// `true` if the user committed (Enter/click away with changed value),
    /// `false` if the user reverted (Escape or no change).
    pub committed: bool,
}
