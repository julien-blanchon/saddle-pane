//! Compile-time [Iconify](https://iconify.design) SVG icons for Bevy.
//!
//! The [`svg!`] macro fetches icons from the Iconify API at compile time and
//! embeds them as `&'static str` constants. At runtime, the SVG is already
//! baked into your binary — no network, no file I/O, no cache lookup.
//!
//! ## Quick start
//!
//! ```ignore
//! const ICON_HOME: &str = bevy_iconify::svg!("mdi:home");
//! const ICON_SWORD: &str = bevy_iconify::svg!("mdi:sword", color = "white", width = "24");
//! ```
//!
//! ## Render with bevy_vello
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_vello::{prelude::*, integrations::svg::load_svg_from_str};
//!
//! const ICON_HOME: &str = bevy_iconify::svg!("mdi:home", color = "white");
//!
//! fn setup(mut commands: Commands, mut svgs: ResMut<Assets<VelloSvg>>) {
//!     let handle = svgs.add(load_svg_from_str(ICON_HOME).unwrap());
//!     commands.spawn((
//!         Node { width: Val::Px(24.0), height: Val::Px(24.0), ..default() },
//!         UiVelloSvg(handle),
//!     ));
//! }
//! ```
//!
//! ## Finding icons
//!
//! ```bash
//! iconify_cli search sword
//! iconify_cli collection mdi --filter home
//! ```
//!
//! See the [README](../README.md) for full documentation.

mod attrs;
mod cache;
mod suggest;
mod svg;

/// Embed an Iconify SVG icon at compile time.
///
/// Returns `&'static str` containing the full SVG markup.
///
/// # Syntax
///
/// ```ignore
/// bevy_iconify::svg!("pack:name")
/// bevy_iconify::svg!("pack:name", color = "red", width = "24")
/// ```
///
/// # Parameters
///
/// All optional:
/// - `color` — replace `currentColor` (e.g. `"red"`, `"#ff0000"`)
/// - `width` / `height` — SVG dimensions (e.g. `"24"`, `"2em"`)
/// - `flip` — `"horizontal"`, `"vertical"`, or `"both"`
/// - `rotate` — `"90"`, `"180"`, or `"270"`
/// - `view_box` — `true` to add an invisible bounding rectangle
///
/// # Examples
///
/// ```ignore
/// let home = bevy_iconify::svg!("mdi:home");
/// let sword = bevy_iconify::svg!("mdi:sword", color = "white");
/// let star = bevy_iconify::svg!("lucide:star", width = "16", height = "16");
/// let shield = bevy_iconify::svg!("game-icons:shield", color = "#4a90d9");
/// let arrow = bevy_iconify::svg!("mdi:arrow-left", flip = "horizontal", rotate = "90");
/// ```
#[proc_macro]
pub fn svg(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match svg::iconify_svg_impl(input.into()) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
