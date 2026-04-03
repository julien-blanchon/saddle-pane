use super::*;

#[test]
fn get_set_float() {
    let mut store = PaneStore::default();
    store.set("pane", "speed", 5.0_f64);
    let val: f64 = store.get("pane", "speed");
    assert!((val - 5.0).abs() < f64::EPSILON);
}

#[test]
fn get_set_f32() {
    let mut store = PaneStore::default();
    store.set("pane", "speed", 3.5_f32);
    let val: f32 = store.get("pane", "speed");
    assert!((val - 3.5).abs() < f32::EPSILON);
}

#[test]
fn get_set_bool() {
    let mut store = PaneStore::default();
    store.set("pane", "god", true);
    let val: bool = store.get("pane", "god");
    assert!(val);
}

#[test]
fn get_set_string() {
    let mut store = PaneStore::default();
    store.set("pane", "name", "hello");
    let val: String = store.get("pane", "name");
    assert_eq!(val, "hello");
}

#[test]
fn get_set_int() {
    let mut store = PaneStore::default();
    store.set("pane", "count", 42_i64);
    let val: i64 = store.get("pane", "count");
    assert_eq!(val, 42);
}

#[test]
fn get_raw_returns_none_for_missing() {
    let store = PaneStore::default();
    assert!(store.get_raw("pane", "missing").is_none());
}

#[test]
fn init_and_reset() {
    let mut store = PaneStore::default();
    store.init("pane", "speed", PaneValue::Float(5.0));
    store.set("pane", "speed", 10.0_f64);
    let val: f64 = store.get("pane", "speed");
    assert!((val - 10.0).abs() < f64::EPSILON);

    store.reset("pane", "speed");
    let val: f64 = store.get("pane", "speed");
    assert!((val - 5.0).abs() < f64::EPSILON);
}

#[test]
#[should_panic(expected = "no value for")]
fn get_missing_panics() {
    let store = PaneStore::default();
    let _: f64 = store.get("pane", "missing");
}

#[test]
fn try_get_returns_none_for_missing() {
    let store = PaneStore::default();
    assert!(store.try_get::<f64>("pane", "missing").is_none());
}

#[test]
fn try_get_returns_none_for_type_mismatch() {
    let mut store = PaneStore::default();
    store.set("pane", "flag", true);
    assert!(store.try_get::<f64>("pane", "flag").is_none());
}

#[test]
fn try_get_returns_value_on_match() {
    let mut store = PaneStore::default();
    store.set("pane", "speed", 5.0_f64);
    assert_eq!(store.try_get::<f64>("pane", "speed"), Some(5.0));
}

#[test]
fn get_or_returns_default_for_missing() {
    let store = PaneStore::default();
    assert!((store.get_or("pane", "missing", 42.0_f64) - 42.0).abs() < f64::EPSILON);
}

#[test]
fn get_or_returns_value_when_present() {
    let mut store = PaneStore::default();
    store.set("pane", "speed", 5.0_f64);
    assert!((store.get_or("pane", "speed", 42.0_f64) - 5.0).abs() < f64::EPSILON);
}

#[test]
fn contains_existing_key() {
    let mut store = PaneStore::default();
    store.set("pane", "speed", 5.0_f64);
    assert!(store.contains("pane", "speed"));
}

#[test]
fn contains_missing_key() {
    let store = PaneStore::default();
    assert!(!store.contains("pane", "missing"));
}
