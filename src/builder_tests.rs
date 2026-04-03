use super::*;
use crate::params::{Monitor, Number, Slider};

#[test]
fn builder_creates_slider() {
    let spec = PaneBuilder::new("Test")
        .slider("Speed", Slider::new(0.0..=10.0, 5.0))
        .build();

    assert_eq!(spec.title, "Test");
    assert_eq!(spec.items.len(), 1);
    match &spec.items[0] {
        LayoutItem::Control(ControlSpec::Slider {
            label,
            min,
            max,
            default,
            ..
        }) => {
            assert_eq!(label, "Speed");
            assert!((*min - 0.0).abs() < f64::EPSILON);
            assert!((*max - 10.0).abs() < f64::EPSILON);
            assert!((*default - 5.0).abs() < f64::EPSILON);
        }
        _ => panic!("Expected Slider"),
    }
}

#[test]
fn builder_creates_folder() {
    let spec = PaneBuilder::new("Test")
        .folder("Physics", |f| {
            f.slider("Gravity", Slider::new(-20.0..=0.0, -9.81))
                .toggle("Enabled", true)
        })
        .build();

    assert_eq!(spec.items.len(), 1);
    match &spec.items[0] {
        LayoutItem::Folder { label, items, .. } => {
            assert_eq!(label, "Physics");
            assert_eq!(items.len(), 2);
        }
        _ => panic!("Expected Folder"),
    }
}

#[test]
fn slider_step_inline() {
    let spec = PaneBuilder::new("Test")
        .slider("Speed", Slider::new(0.0..=10.0, 5.0).step(0.5))
        .build();

    match &spec.items[0] {
        LayoutItem::Control(ControlSpec::Slider { step, .. }) => {
            assert!((step - 0.5).abs() < f64::EPSILON);
        }
        _ => panic!("Expected Slider"),
    }
}

#[test]
fn slider_with_tooltip_and_icon() {
    let spec = PaneBuilder::new("Test")
        .slider("Speed", Slider::new(0.0..=10.0, 5.0).step(0.1).tooltip("help").icon("svg"))
        .build();

    match &spec.items[0] {
        LayoutItem::Control(ControlSpec::Slider { tooltip, icon, step, .. }) => {
            assert_eq!(tooltip.as_deref(), Some("help"));
            assert_eq!(icon.as_deref(), Some("svg"));
            assert!((step - 0.1).abs() < f64::EPSILON);
        }
        _ => panic!("Expected Slider"),
    }
}

#[test]
fn multiple_controls() {
    let spec = PaneBuilder::new("Debug")
        .slider("Speed", Slider::new(0.0..=10.0, 5.0))
        .toggle("God Mode", false)
        .button("Reset")
        .separator()
        .number("Score", Number::new(100.0))
        .text("Name", "Player")
        .select("Quality", &["Low", "Medium", "High"], 1)
        .color("Tint", Color::WHITE)
        .build();

    assert_eq!(spec.items.len(), 8);
}

#[test]
fn number_with_config() {
    let spec = PaneBuilder::new("Test")
        .number("Score", Number::new(100.0).step(1.0).min(0.0).max(999.0))
        .build();

    match &spec.items[0] {
        LayoutItem::Control(ControlSpec::Number { default, step, min, max, .. }) => {
            assert!((*default - 100.0).abs() < f64::EPSILON);
            assert!((*step - 1.0).abs() < f64::EPSILON);
            assert_eq!(*min, Some(0.0));
            assert_eq!(*max, Some(999.0));
        }
        _ => panic!("Expected Number"),
    }
}

#[test]
fn monitor_variants() {
    let spec = PaneBuilder::new("Test")
        .monitor("FPS", Monitor::text("—"))
        .monitor("Log", Monitor::log(8))
        .monitor("CPU", Monitor::graph(0.0..=100.0, 64))
        .build();

    assert_eq!(spec.items.len(), 3);
    assert!(matches!(&spec.items[0], LayoutItem::Control(ControlSpec::Monitor { .. })));
    assert!(matches!(&spec.items[1], LayoutItem::Control(ControlSpec::MonitorLog { .. })));
    assert!(matches!(&spec.items[2], LayoutItem::Control(ControlSpec::MonitorGraph { .. })));
}

#[test]
fn tab_api() {
    let spec = PaneBuilder::new("Test")
        .tab("General", |p| p.toggle("A", true))
        .tab("Physics", |p| p.toggle("B", false))
        .build();

    assert_eq!(spec.items.len(), 1);
    match &spec.items[0] {
        LayoutItem::TabGroup { tabs, .. } => {
            assert_eq!(tabs.len(), 2);
            assert_eq!(tabs[0].label, "General");
            assert_eq!(tabs[1].label, "Physics");
        }
        _ => panic!("Expected TabGroup"),
    }
}

#[test]
fn footer_api() {
    let spec = PaneBuilder::new("Test")
        .toggle("A", true)
        .footer(|f| f.button("Reset").button("Save"))
        .build();

    assert_eq!(spec.items.len(), 1);
    assert_eq!(spec.footer.len(), 2);
}

#[test]
fn pane_config() {
    let spec = PaneBuilder::new("Test")
        .position(100.0, 50.0)
        .width(320.0)
        .collapsed(true)
        .build();

    assert_eq!(spec.position, Some(PanePosition::Absolute(100.0, 50.0)));
    assert_eq!(spec.width, Some(320.0));
    assert!(spec.collapsed);
}
