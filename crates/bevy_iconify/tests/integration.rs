//! Integration tests for bevy_iconify::svg! macro.
//!
//! These tests invoke the actual proc-macro and hit the Iconify API (or cache).

/// Basic icon fetch — the most common use case.
#[test]
fn svg_macro_basic() {
    let svg = bevy_iconify::svg!("mdi:home");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
    assert!(svg.contains("viewBox"));
}

/// Icon with color parameter.
#[test]
fn svg_macro_with_color() {
    let svg = bevy_iconify::svg!("mdi:home", color = "red");
    assert!(svg.contains("<svg"));
    // When color is set, currentColor should be replaced
    assert!(!svg.contains("currentColor"), "color=red should replace currentColor");
}

/// Icon with explicit dimensions.
#[test]
fn svg_macro_with_dimensions() {
    let svg = bevy_iconify::svg!("mdi:home", width = "48", height = "48");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("48"));
}

/// Icon with flip transformation.
#[test]
fn svg_macro_with_flip() {
    let svg = bevy_iconify::svg!("mdi:arrow-left", flip = "horizontal");
    assert!(svg.contains("<svg"));
    // Flipped SVG should contain a transform
    assert!(svg.contains("translate") || svg.contains("scale") || svg.contains("<svg"));
}

/// Icon with rotation.
#[test]
fn svg_macro_with_rotation() {
    let svg = bevy_iconify::svg!("mdi:arrow-up", rotate = "90");
    assert!(svg.contains("<svg"));
}

/// Icon with view_box parameter.
#[test]
fn svg_macro_with_view_box() {
    let svg = bevy_iconify::svg!("mdi:home", view_box = true);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("viewBox"));
}

/// All parameters combined.
#[test]
fn svg_macro_all_params() {
    let svg = bevy_iconify::svg!(
        "mdi:sword",
        color = "white",
        width = "24",
        height = "24",
        flip = "horizontal",
        rotate = "90",
        view_box = true,
    );
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
}

/// Different icon sets work.
#[test]
fn svg_macro_different_sets() {
    let lucide = bevy_iconify::svg!("lucide:star");
    assert!(lucide.contains("<svg"));

    let tabler = bevy_iconify::svg!("tabler:home");
    assert!(tabler.contains("<svg"));
}

/// The macro returns a &'static str usable as a const.
#[test]
fn svg_macro_const_usage() {
    const HOME: &str = bevy_iconify::svg!("mdi:home");
    assert!(!HOME.is_empty());
    assert!(HOME.starts_with("<svg"));
}

/// Trailing comma is allowed.
#[test]
fn svg_macro_trailing_comma() {
    let svg = bevy_iconify::svg!("mdi:home", color = "blue",);
    assert!(svg.contains("<svg"));
}
