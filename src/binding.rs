//! Resource ↔ pane bidirectional sync.
//!
//! Used by the `#[derive(Pane)]` macro to automatically generate pane UI
//! from a `Resource` struct and keep it in sync — no `Reflect` needed.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::builder::PaneBuilder;
use crate::controls::PaneControlMeta;
use crate::controls::PaneValue;
use crate::controls::color::ColorControl;
use crate::controls::monitor::MonitorControl;
use crate::controls::number::NumberControl;
use crate::controls::select::SelectControl;
use crate::controls::slider::{SliderControl, SliderWidgetLink};
use crate::controls::text::TextControl;
use crate::controls::toggle::ToggleControl;

/// Trait implemented by `#[derive(Pane)]`. Describes the pane structure
/// and provides direct field access — no `Reflect` required.
pub trait PaneDerive: Resource {
    fn pane_title() -> &'static str;
    fn field_descriptors() -> Vec<FieldDescriptor>;

    /// Optional position for the spawned pane. `None` uses default (top-right).
    fn pane_position() -> Option<crate::builder::PanePosition> { None }

    /// Read a field value by name. Returns `None` if the field doesn't exist.
    fn read_field(&self, field_name: &str) -> Option<PaneValue>;

    /// Write a field value by name. Returns `true` if the value was updated.
    fn write_field(&mut self, field_name: &str, value: &PaneValue) -> bool;
}

/// Description of a single field's control.
#[derive(Clone, Debug)]
pub struct FieldDescriptor {
    pub field_name: String,
    pub label: String,
    pub folder: Option<String>,
    pub tab: Option<String>,
    pub tooltip: Option<String>,
    pub icon: Option<String>,
    pub order: i32,
    pub kind: FieldControlKind,
}

/// The type of control for a field.
#[derive(Clone, Debug)]
pub enum FieldControlKind {
    Slider { min: f64, max: f64, step: f64 },
    Number { min: Option<f64>, max: Option<f64>, step: f64 },
    Toggle,
    Text,
    Color,
    Select { options: Vec<String> },
    Monitor,
    /// Custom plugin control. `control_id` matches the plugin's registered ID.
    /// Config values are passed as key-value pairs.
    Custom {
        control_id: String,
        config: Vec<(String, f64)>,
    },
}

/// Marker component on a PaneRoot that was spawned from a derive-pane.
#[derive(Component, Debug)]
pub struct DerivePane<T: PaneDerive> {
    #[allow(dead_code)]
    _marker: std::marker::PhantomData<T>,
}

/// Tracks whether `sync_pane_to_resource` caused the last resource mutation.
/// Prevents feedback loops: when we write to the resource, the reverse sync
/// (`sync_resource_to_pane`) skips the resulting `Changed<T>`.
#[derive(Resource, Default)]
struct DeriveSyncFlag {
    wrote_to_resource: bool,
}

/// Extension trait on `App` to register a derive-pane resource.
pub trait RegisterPaneExt {
    /// Register a `#[derive(Pane)]` resource. Initializes the resource (if not
    /// already present), spawns the pane UI, and adds bidirectional sync systems.
    fn register_pane<T: PaneDerive + FromWorld>(&mut self) -> &mut Self;
}

impl RegisterPaneExt for App {
    fn register_pane<T: PaneDerive + FromWorld>(&mut self) -> &mut Self {
        self.init_resource::<T>();
        self.init_resource::<DeriveSyncFlag>();
        self.add_systems(Startup, spawn_derive_pane::<T>);
        self.add_systems(
            PostUpdate,
            (
                sync_pane_to_resource::<T>,
                sync_resource_to_pane::<T>,
            )
                .chain(),
        );
        self
    }
}

/// Startup system: spawn the pane from the derive descriptor.
fn spawn_derive_pane<T: PaneDerive>(resource: Res<T>, mut commands: Commands) {
    let title = T::pane_title();
    let descriptors = T::field_descriptors();

    // Group fields by container: root, folder, or tab
    let mut root_fields: Vec<&FieldDescriptor> = Vec::new();
    let mut folders: HashMap<String, Vec<&FieldDescriptor>> = HashMap::new();
    let mut tabs: HashMap<String, Vec<&FieldDescriptor>> = HashMap::new();

    for desc in &descriptors {
        if let Some(ref tab) = desc.tab {
            tabs.entry(tab.clone()).or_default().push(desc);
        } else if let Some(ref folder) = desc.folder {
            folders.entry(folder.clone()).or_default().push(desc);
        } else {
            root_fields.push(desc);
        }
    }

    let mut builder = PaneBuilder::new(title);
    if let Some(pos) = T::pane_position() {
        builder = builder.at(pos);
    }

    // Tabs (sorted by minimum order of their fields)
    if !tabs.is_empty() {
        let mut tab_order: Vec<(String, Vec<&FieldDescriptor>)> = tabs.into_iter().collect();
        tab_order.sort_by_key(|(_, fields)| fields.iter().map(|d| d.order).min().unwrap_or(0));
        for (tab_name, mut fields) in tab_order {
            fields.sort_by_key(|d| d.order);
            let res_ref = &*resource;
            let fields_clone: Vec<FieldDescriptor> = fields.into_iter().cloned().collect();
            builder = builder.tab(&tab_name, move |mut p| {
                for desc in &fields_clone {
                    p = add_field_to_folder(p, desc, res_ref);
                }
                p
            });
        }
    }

    // Root-level fields (sorted by order)
    root_fields.sort_by_key(|d| d.order);
    for desc in &root_fields {
        builder = add_field_to_builder(builder, desc, &*resource);
    }

    // Folders (sorted by minimum field order)
    let mut folder_order: Vec<(String, Vec<&FieldDescriptor>)> = folders.into_iter().collect();
    folder_order.sort_by_key(|(_, fields)| fields.iter().map(|d| d.order).min().unwrap_or(0));
    for (folder_name, mut fields) in folder_order {
        fields.sort_by_key(|d| d.order);
        let res_ref = &*resource;
        let fields_clone: Vec<FieldDescriptor> = fields.into_iter().cloned().collect();
        builder = builder.folder(&folder_name, move |mut f| {
            for desc in &fields_clone {
                f = add_field_to_folder(f, desc, res_ref);
            }
            f
        });
    }

    let pane_entity = builder.spawn(&mut commands);

    commands.entity(pane_entity).insert(DerivePane::<T> {
        _marker: std::marker::PhantomData,
    });
}

/// Get a default value for a field from the resource, falling back to the descriptor kind's default.
fn field_default_f64(resource: &dyn PaneDeriveReader, desc: &FieldDescriptor, fallback: f64) -> f64 {
    resource.read_field(&desc.field_name).and_then(|v| match v {
        PaneValue::Float(f) => Some(f),
        PaneValue::Int(i) => Some(i as f64),
        _ => None,
    }).unwrap_or(fallback)
}

/// Trait alias for reading fields (avoids generic constraints in helpers).
trait PaneDeriveReader {
    fn read_field(&self, field_name: &str) -> Option<PaneValue>;
}
impl<T: PaneDerive> PaneDeriveReader for T {
    fn read_field(&self, field_name: &str) -> Option<PaneValue> {
        PaneDerive::read_field(self, field_name)
    }
}

/// Build a control from a FieldDescriptor onto any builder that has the control methods.
macro_rules! add_field {
    ($builder:expr, $desc:expr, $resource:expr) => {{
        use crate::params;

        let desc: &FieldDescriptor = $desc;
        let resource: &dyn PaneDeriveReader = $resource;

        match &desc.kind {
            FieldControlKind::Slider { min, max, step } => {
                let default = field_default_f64(resource, desc, (*min + *max) / 2.0);
                let mut s = params::Slider::new(*min..=*max, default).step(*step);
                if let Some(ref tip) = desc.tooltip { s = s.tooltip(tip.as_str()); }
                if let Some(ref icon) = desc.icon { s = s.icon(icon); }
                $builder.slider(&desc.label, s)
            }
            FieldControlKind::Number { min, max, step } => {
                let default = field_default_f64(resource, desc, 0.0);
                let mut n = params::Number::new(default).step(*step);
                if let Some(mn) = min { n = n.min(*mn); }
                if let Some(mx) = max { n = n.max(*mx); }
                if let Some(ref tip) = desc.tooltip { n = n.tooltip(tip.as_str()); }
                if let Some(ref icon) = desc.icon { n = n.icon(icon); }
                $builder.number(&desc.label, n)
            }
            FieldControlKind::Toggle => {
                let default = resource.read_field(&desc.field_name)
                    .and_then(|v| if let PaneValue::Bool(b) = v { Some(b) } else { None })
                    .unwrap_or(false);
                if desc.tooltip.is_some() || desc.icon.is_some() {
                    let mut t = params::Toggle::new(default);
                    if let Some(ref tip) = desc.tooltip { t = t.tooltip(tip.as_str()); }
                    if let Some(ref icon) = desc.icon { t = t.icon(icon); }
                    $builder.toggle_opts(&desc.label, t)
                } else {
                    $builder.toggle(&desc.label, default)
                }
            }
            FieldControlKind::Text => {
                let default = resource.read_field(&desc.field_name)
                    .and_then(|v| if let PaneValue::String(s) = v { Some(s) } else { None })
                    .unwrap_or_default();
                if desc.tooltip.is_some() || desc.icon.is_some() {
                    let mut t = params::TextInput::new(&default);
                    if let Some(ref tip) = desc.tooltip { t = t.tooltip(tip.as_str()); }
                    if let Some(ref icon) = desc.icon { t = t.icon(icon); }
                    $builder.text_opts(&desc.label, t)
                } else {
                    $builder.text(&desc.label, &default)
                }
            }
            FieldControlKind::Color => {
                let default = resource.read_field(&desc.field_name)
                    .and_then(|v| if let PaneValue::Color(c) = v { Some(c) } else { None })
                    .unwrap_or(Color::WHITE);
                if desc.tooltip.is_some() || desc.icon.is_some() {
                    let mut c = params::ColorPicker::new(default);
                    if let Some(ref tip) = desc.tooltip { c = c.tooltip(tip.as_str()); }
                    if let Some(ref icon) = desc.icon { c = c.icon(icon); }
                    $builder.color_opts(&desc.label, c)
                } else {
                    $builder.color(&desc.label, default)
                }
            }
            FieldControlKind::Select { options } => {
                let default = resource.read_field(&desc.field_name)
                    .and_then(|v| match v {
                        PaneValue::Int(i) => Some(i as usize),
                        PaneValue::Float(f) => Some(f as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                let opts: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
                if desc.tooltip.is_some() || desc.icon.is_some() {
                    let mut s = params::SelectMenu::new(&opts, default);
                    if let Some(ref tip) = desc.tooltip { s = s.tooltip(tip.as_str()); }
                    if let Some(ref icon) = desc.icon { s = s.icon(icon); }
                    $builder.select_opts(&desc.label, s)
                } else {
                    $builder.select(&desc.label, &opts, default)
                }
            }
            FieldControlKind::Monitor => {
                $builder.monitor(&desc.label, params::Monitor::text(""))
            }
            FieldControlKind::Custom { control_id, config } => {
                let mut ctrl_config = crate::registry::ControlConfig::new();
                for (key, val) in config {
                    ctrl_config = ctrl_config.float(key, *val);
                }
                $builder.custom(control_id, &desc.label, ctrl_config)
            }
        }
    }};
}

fn add_field_to_builder(
    builder: PaneBuilder,
    desc: &FieldDescriptor,
    resource: &dyn PaneDeriveReader,
) -> PaneBuilder {
    add_field!(builder, desc, resource)
}

fn add_field_to_folder(
    folder: crate::builder::FolderBuilder,
    desc: &FieldDescriptor,
    resource: &dyn PaneDeriveReader,
) -> crate::builder::FolderBuilder {
    add_field!(folder, desc, resource)
}

/// Build a label→entity lookup map for controls belonging to a specific pane.
fn build_entity_map<'a>(
    q_meta: &'a Query<(Entity, &PaneControlMeta)>,
    title: &str,
) -> HashMap<&'a str, Entity> {
    q_meta
        .iter()
        .filter(|(_, meta)| meta.pane_title == title)
        .map(|(e, meta)| (meta.label.as_str(), e))
        .collect()
}

/// System: sync resource fields → pane controls when the resource changes externally.
/// Skips if the change was caused by our own `sync_pane_to_resource` (flag-based guard).
fn sync_resource_to_pane<T: PaneDerive>(
    resource: Res<T>,
    mut sync_flag: ResMut<DeriveSyncFlag>,
    q_meta: Query<(Entity, &PaneControlMeta)>,
    mut q_sliders: Query<(&mut SliderControl, &SliderWidgetLink)>,
    mut q_toggles: Query<&mut ToggleControl, Without<SliderControl>>,
    mut q_numbers: Query<&mut NumberControl, (Without<SliderControl>, Without<ToggleControl>)>,
    mut q_texts: Query<
        &mut TextControl,
        (Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>),
    >,
    mut q_selects: Query<
        &mut SelectControl,
        (Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>, Without<TextControl>),
    >,
    mut q_colors: Query<
        &mut ColorControl,
        (Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>, Without<TextControl>, Without<SelectControl>),
    >,
    mut q_monitors: Query<
        &mut MonitorControl,
        (Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>, Without<TextControl>, Without<SelectControl>, Without<ColorControl>),
    >,
    mut commands: Commands,
) {
    if sync_flag.wrote_to_resource {
        sync_flag.wrote_to_resource = false;
        return;
    }

    if !resource.is_changed() {
        return;
    }

    let title = T::pane_title();
    let descriptors = T::field_descriptors();
    let entity_map = build_entity_map(&q_meta, title);

    for desc in &descriptors {
        let Some(&control_entity) = entity_map.get(desc.label.as_str()) else {
            continue;
        };

        let Some(field_value) = resource.read_field(&desc.field_name) else {
            continue;
        };

        // Use f32-precision comparison to avoid feedback loops when the resource
        // field is f32 (round-trip f64→f32→f64 introduces tiny differences).
        match (&desc.kind, &field_value) {
            (FieldControlKind::Slider { .. }, PaneValue::Float(v)) => {
                if let Ok((mut ctrl, link)) = q_sliders.get_mut(control_entity) {
                    if (ctrl.value as f32 - *v as f32).abs() > f32::EPSILON {
                        ctrl.value = *v;
                        commands.entity(link.0).insert(bevy_ui_widgets::SliderValue(*v as f32));
                    }
                }
            }
            (FieldControlKind::Number { .. }, PaneValue::Float(v)) => {
                if let Ok(mut ctrl) = q_numbers.get_mut(control_entity) {
                    if (ctrl.value as f32 - *v as f32).abs() > f32::EPSILON {
                        ctrl.value = *v;
                    }
                }
            }
            (FieldControlKind::Toggle, PaneValue::Bool(v)) => {
                if let Ok(mut ctrl) = q_toggles.get_mut(control_entity) {
                    if ctrl.value != *v {
                        ctrl.value = *v;
                    }
                }
            }
            (FieldControlKind::Text, PaneValue::String(v)) => {
                if let Ok(mut ctrl) = q_texts.get_mut(control_entity) {
                    if ctrl.value != *v {
                        ctrl.value.clone_from(v);
                    }
                }
            }
            (FieldControlKind::Color, PaneValue::Color(v)) => {
                if let Ok(mut ctrl) = q_colors.get_mut(control_entity) {
                    if ctrl.value != *v {
                        ctrl.value = *v;
                    }
                }
            }
            (FieldControlKind::Select { .. }, PaneValue::Int(v)) => {
                if let Ok(mut ctrl) = q_selects.get_mut(control_entity) {
                    let idx = *v as usize;
                    if ctrl.value != idx {
                        ctrl.value = idx;
                    }
                }
            }
            (FieldControlKind::Monitor, PaneValue::String(v)) => {
                if let Ok(mut ctrl) = q_monitors.get_mut(control_entity) {
                    if ctrl.value != *v {
                        ctrl.value.clone_from(v);
                    }
                }
            }
            _ => {}
        }
    }
}

/// System: sync pane controls → resource when the user interacts with the UI.
/// Only mutates the resource when at least one control has actually changed.
fn sync_pane_to_resource<T: PaneDerive>(
    mut resource: ResMut<T>,
    mut sync_flag: ResMut<DeriveSyncFlag>,
    q_meta: Query<(Entity, &PaneControlMeta)>,
    q_sliders: Query<&SliderControl, Changed<SliderControl>>,
    q_toggles: Query<&ToggleControl, (Changed<ToggleControl>, Without<SliderControl>)>,
    q_numbers: Query<
        &NumberControl,
        (Changed<NumberControl>, Without<SliderControl>, Without<ToggleControl>),
    >,
    q_texts: Query<
        &TextControl,
        (Changed<TextControl>, Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>),
    >,
    q_selects: Query<
        &SelectControl,
        (Changed<SelectControl>, Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>, Without<TextControl>),
    >,
    q_colors: Query<
        &ColorControl,
        (Changed<ColorControl>, Without<SliderControl>, Without<ToggleControl>, Without<NumberControl>, Without<TextControl>, Without<SelectControl>),
    >,
) {
    let title = T::pane_title();
    let descriptors = T::field_descriptors();
    let entity_map = build_entity_map(&q_meta, title);

    // First pass: collect changed values without touching the resource.
    let mut updates: Vec<(&str, PaneValue)> = Vec::new();

    for desc in &descriptors {
        let Some(&control_entity) = entity_map.get(desc.label.as_str()) else {
            continue;
        };

        let value = match &desc.kind {
            FieldControlKind::Slider { .. } => {
                q_sliders.get(control_entity).ok().map(|c| PaneValue::Float(c.value))
            }
            FieldControlKind::Number { .. } => {
                q_numbers.get(control_entity).ok().map(|c| PaneValue::Float(c.value))
            }
            FieldControlKind::Toggle => {
                q_toggles.get(control_entity).ok().map(|c| PaneValue::Bool(c.value))
            }
            FieldControlKind::Text => {
                q_texts.get(control_entity).ok().map(|c| PaneValue::String(c.value.clone()))
            }
            FieldControlKind::Color => {
                q_colors.get(control_entity).ok().map(|c| PaneValue::Color(c.value))
            }
            FieldControlKind::Select { .. } => {
                q_selects.get(control_entity).ok().map(|c| PaneValue::Int(c.value as i64))
            }
            FieldControlKind::Monitor | FieldControlKind::Custom { .. } => None,
        };

        if let Some(v) = value {
            updates.push((&desc.field_name, v));
        }
    }

    if updates.is_empty() {
        return;
    }

    // Second pass: apply all changes to the resource in one mutation.
    sync_flag.wrote_to_resource = true;
    let res = resource.as_mut();
    for (field_name, value) in updates {
        res.write_field(field_name, &value);
    }
}
