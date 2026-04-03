//! # saddle_pane
//!
//! Lightweight debug/tweaking panel for Bevy 0.18.
//!
//! ## Recommended: Derive API
//!
//! ```rust,ignore
//! #[derive(Resource, Default, Pane)]
//! #[pane(title = "Settings")]
//! struct Settings {
//!     #[pane(slider, min = 0.0, max = 10.0)]
//!     speed: f32,
//!     enabled: bool, // auto-detected as toggle
//! }
//!
//! app.register_pane::<Settings>(); // inits resource + spawns UI + sync
//! ```
//!
//! ## Alternative: Builder API (prototyping)
//!
//! ```rust,ignore
//! PaneBuilder::new("Debug")
//!     .slider("Speed", Slider::new(0.0..=10.0, 5.0))
//!     .spawn(&mut commands);
//! ```

pub mod binding;
pub mod builder;
pub mod controls;
pub mod params;
pub mod events;
pub mod handle;
pub mod icons;
pub mod layout;
mod plugin;
pub mod registry;
pub mod store;
mod style;
mod search;
mod sync;
mod ux;
pub mod theme;

pub use plugin::{PanePlugin, PaneSystems};

#[cfg(feature = "derive")]
pub use saddle_pane_derive::Pane;

/// Prelude for convenient imports.
pub mod prelude {
    #[cfg(feature = "derive")]
    pub use crate::binding::RegisterPaneExt;
    pub use crate::builder::{PaneBuilder, PanePosition};
    pub use crate::params::{
        ButtonOpts, ColorPicker, Monitor, Number, SelectMenu, Slider, TextInput, Toggle,
    };
    pub use crate::controls::PaneValue;
    pub use crate::controls::monitor::{MonitorControl, MonitorGraphControl, MonitorLogControl};
    pub use crate::events::{PaneButtonPressed, PaneChanged, PaneEditEnd, PaneEditStart};
    pub use crate::icons::{spawn_pane_icon, spawn_pane_icon_handle, PaneIconPlaceholder};
    pub use crate::layout::{PaneFolder, PaneRoot};
    pub use crate::plugin::{PanePlugin, PaneSystems};
    pub use crate::registry::{
        ControlConfig, PaneControlPlugin, PaneControlRegistry, PaneCustomValue,
    };
    pub use crate::handle::PaneHandle;
    pub use crate::store::{FromPaneValue, IntoPaneValue, PaneStore};
    pub use crate::theme::{PaneTheme, PaneThemeOverride, PaneThemePreset};

    #[cfg(feature = "derive")]
    pub use crate::Pane;
}
