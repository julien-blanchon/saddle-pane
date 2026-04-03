//! Color picker popup with HSV color plane + hue slider.
//!
//! When the color swatch is clicked, a popup appears with:
//! - A 2D Saturation-Value plane (shader rendered)
//! - A hue slider bar (rainbow gradient)
//! - A hex value display

use bevy::picking::Pickable;
use bevy::picking::events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy_flair::prelude::{ClassList, InlineStyle};
use bevy_flair::style::components::NodeStyleSheet;
use bevy_ui_widgets::{Slider, SliderRange, SliderThumb, SliderValue, TrackClick};

use super::color::{ColorControl, ColorControlMarker, ColorSwatch};
use super::editing::color_to_hex;
use super::value_font;

pub(crate) const STYLE_PATH: &str = "embedded://saddle_pane/style/color_picker.css";

fn css_percent(value: f32) -> String {
    format!("{:.4}%", value.clamp(0.0, 100.0))
}

// ── Shader Material ────────────────────────────────────────

/// Uniform data for the HSV color plane shader.
#[derive(AsBindGroup, Asset, TypePath, Default, Debug, Clone)]
pub(crate) struct HsvPlaneMaterial {
    #[uniform(0)]
    hue: f32,
}

impl UiMaterial for HsvPlaneMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://saddle_pane/style/color_picker.wgsl".into()
    }
}

// ── Components ─────────────────────────────────────────────

/// Marker for the color picker popup entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorPickerPopup;

/// Links a popup to its owning color control row.
#[derive(Component, Clone, Debug)]
pub(crate) struct ColorPickerOwner(pub Entity);

/// Links a popup to the wrapper entity it should be positioned relative to.
#[derive(Component, Clone, Debug)]
pub(crate) struct ColorPickerAnchor(pub Entity);

/// Marker for the SV plane inner area (receives pointer events).
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorPlaneInner;

/// Marker for the SV plane thumb.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorPlaneThumb;

/// Tracks whether the plane is being dragged.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorPlaneDragState(pub bool);

/// Marker for the hue slider entity.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct HueSliderMarker;

/// Marker for the hex text in the picker.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct ColorPickerHexText;

/// Whether the color picker popup is open for a given color control.
#[derive(Component, Clone, Debug, Default)]
pub struct ColorPickerOpen(pub bool);

/// Cached HSV state for the color picker.
#[derive(Component, Clone, Debug)]
pub(crate) struct ColorPickerHsv {
    pub hue: f32,        // 0..360
    pub saturation: f32, // 0..1
    pub value: f32,      // 0..1
}

impl Default for ColorPickerHsv {
    fn default() -> Self {
        Self {
            hue: 0.0,
            saturation: 1.0,
            value: 1.0,
        }
    }
}

// ── HSV ↔ RGB conversion ──────────────────────────────────

fn rgb_to_hsv(color: Color) -> (f32, f32, f32) {
    let srgba = color.to_srgba();
    let r = srgba.red;
    let g = srgba.green;
    let b = srgba.blue;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta < 1e-6 {
        0.0
    } else if (max - r).abs() < 1e-6 {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < 1e-6 {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max < 1e-6 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v)
}

fn hsv_to_color(h: f32, s: f32, v: f32) -> Color {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if hp < 1.0 {
        (c, x, 0.0)
    } else if hp < 2.0 {
        (x, c, 0.0)
    } else if hp < 3.0 {
        (0.0, c, x)
    } else if hp < 4.0 {
        (0.0, x, c)
    } else if hp < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color::srgb(r + m, g + m, b + m)
}

// ── Spawn ──────────────────────────────────────────────────

/// Spawn the color picker popup as a ROOT entity (not inside the pane hierarchy).
/// This avoids clipping from the pane body's overflow: scroll/clip.
pub(crate) fn spawn_color_picker_popup(
    commands: &mut Commands,
    row_entity: Entity,
    wrapper_entity: Entity,
    color: Color,
    asset_server: &AssetServer,
    materials: &mut Assets<HsvPlaneMaterial>,
) {
    let (hue, sat, val) = rgb_to_hsv(color);
    let hex = color_to_hex(color);

    let material = materials.add(HsvPlaneMaterial { hue });

    // Spawn as root entity — positioned by `position_color_picker` system each frame
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(200.0),
                ..default()
            },
            ColorPickerPopup,
            ColorPickerOwner(row_entity),
            ColorPickerAnchor(wrapper_entity),
            ColorPickerHsv {
                hue,
                saturation: sat,
                value: val,
            },
            ColorPlaneDragState::default(),
            ClassList::new("pane-color-picker-popup"),
            NodeStyleSheet::new(asset_server.load(STYLE_PATH)),
            InlineStyle::from_iter([
                ("--color-picker-thumb-left", css_percent(sat * 100.0)),
                ("--color-picker-thumb-top", css_percent((1.0 - val) * 100.0)),
                ("--hue-slider-thumb-left", css_percent(hue / 360.0 * 100.0)),
            ]),
            GlobalZIndex(1001),
        ))
        .with_children(|popup| {
            // SV Color plane (shader rendered)
            popup
                .spawn((Node::default(), ClassList::new("pane-color-plane")))
                .with_children(|plane_wrapper| {
                    plane_wrapper
                        .spawn((
                            Node::default(),
                            Interaction::default(),
                            ColorPlaneInner,
                            ClassList::new("pane-color-plane-inner"),
                            MaterialNode(material),
                        ))
                        .with_children(|inner| {
                            inner.spawn((
                                Node::default(),
                                ColorPlaneThumb,
                                ClassList::new("pane-color-plane-thumb"),
                                Pickable::IGNORE,
                            ));
                        });
                });

            // Hue slider (rainbow gradient background in CSS)
            popup
                .spawn((
                    Node::default(),
                    Interaction::default(),
                    Slider {
                        track_click: TrackClick::Drag,
                    },
                    SliderValue(hue / 360.0),
                    SliderRange::new(0.0, 1.0),
                    HueSliderMarker,
                    ClassList::new("pane-hue-slider"),
                ))
                .with_children(|slider| {
                    slider
                        .spawn((Node::default(), ClassList::new("pane-hue-slider-track")))
                        .with_children(|track| {
                            track.spawn((
                                Node::default(),
                                Interaction::default(),
                                SliderThumb,
                                ClassList::new("pane-hue-slider-thumb"),
                            ));
                        });
                });

            // Hex value display
            popup
                .spawn((Node::default(), ClassList::new("pane-color-picker-values")))
                .with_children(|vals| {
                    vals.spawn((Node::default(), ClassList::new("pane-color-picker-hex")))
                        .with_children(|hex_box| {
                            hex_box.spawn((
                                Text::new(hex),
                                value_font(),
                                ClassList::new("pane-color-picker-hex-text"),
                                ColorPickerHexText,
                            ));
                        });
                });
        });
}

/// System (PostUpdate): position color picker popup below its anchor wrapper.
pub(crate) fn position_color_picker(
    mut q_popups: Query<(&ColorPickerAnchor, &mut Node), With<ColorPickerPopup>>,
    q_anchor: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &ComputedUiRenderTargetInfo,
        ),
        With<super::color::ColorAreaWrapper>,
    >,
    ui_scale: Res<UiScale>,
) {
    for (anchor, mut node) in &mut q_popups {
        let Ok((computed, transform, target_info)) = q_anchor.get(anchor.0) else {
            continue;
        };
        // UiGlobalTransform gives physical pixel coords.
        // Node left/top are in logical pixels.
        // Combined scale = window_scale_factor * ui_scale
        let combined_scale = target_info.scale_factor() * ui_scale.0;

        let center_px = transform.transform_point2(Vec2::ZERO);
        let half_size_px = computed.size() / 2.0;

        // Convert physical → logical
        let left = (center_px.x - half_size_px.x) / combined_scale;
        let bottom = (center_px.y + half_size_px.y) / combined_scale;

        node.left = Val::Px(left);
        node.top = Val::Px(bottom + 2.0);
    }
}

// ── Systems ────────────────────────────────────────────────

/// System: toggle color picker popup open/close when swatch is clicked.
pub(crate) fn handle_color_picker_toggle(
    q_swatch: Query<(&Interaction, &ChildOf), (Changed<Interaction>, With<ColorSwatch>)>,
    q_parent: Query<&ChildOf>,
    mut q_picker_open: Query<&mut ColorPickerOpen, With<ColorControlMarker>>,
) {
    for (interaction, child_of) in &q_swatch {
        if *interaction == Interaction::Pressed {
            // Swatch → area wrapper → row
            let area_entity = child_of.parent();
            if let Ok(area_parent) = q_parent.get(area_entity) {
                let row_entity = area_parent.parent();
                if let Ok(mut open) = q_picker_open.get_mut(row_entity) {
                    open.0 = !open.0;
                }
            }
        }
    }
}

/// System: sync ColorPickerOpen → spawn/despawn popup.
pub(crate) fn sync_color_picker_open(
    q_colors: Query<
        (Entity, &ColorPickerOpen, &ColorControl),
        (With<ColorControlMarker>, Changed<ColorPickerOpen>),
    >,
    q_popup: Query<(Entity, &ColorPickerOwner)>,
    q_children: Query<&Children>,
    q_wrapper: Query<Entity, With<super::color::ColorAreaWrapper>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<HsvPlaneMaterial>>,
    mut commands: Commands,
) {
    for (row_entity, open, control) in &q_colors {
        let popup_entity = q_popup
            .iter()
            .find(|(_, owner)| owner.0 == row_entity)
            .map(|(e, _)| e);

        match (open.0, popup_entity) {
            (true, None) => {
                // Find the color area wrapper child
                if let Ok(children) = q_children.get(row_entity) {
                    for child in children.iter() {
                        if q_wrapper.contains(child) {
                            spawn_color_picker_popup(
                                &mut commands,
                                row_entity,
                                child,
                                control.value,
                                &asset_server,
                                &mut materials,
                            );
                            break;
                        }
                    }
                }
            }
            (false, Some(popup)) => {
                commands.entity(popup).despawn();
            }
            _ => {}
        }
    }
}

/// Observer: pointer press on the SV plane — set saturation/value.
pub(crate) fn on_plane_press(
    mut press: On<Pointer<Press>>,
    q_inner: Query<(&ComputedNode, &UiGlobalTransform, &ChildOf), With<ColorPlaneInner>>,
    q_parent: Query<&ChildOf>,
    mut q_popup: Query<
        (&mut ColorPickerHsv, &mut InlineStyle, &ColorPickerOwner),
        With<ColorPickerPopup>,
    >,
    mut q_color: Query<&mut ColorControl>,
    ui_scale: Res<UiScale>,
    q_target: Query<&ComputedUiRenderTargetInfo>,
) {
    let Ok((node, transform, inner_parent)) = q_inner.get(press.entity) else {
        return;
    };
    press.propagate(false);

    let plane_wrapper = inner_parent.parent();
    let Ok(wrapper_parent) = q_parent.get(plane_wrapper) else {
        return;
    };
    let popup_entity = wrapper_parent.parent();

    let Ok((mut hsv, mut style, owner)) = q_popup.get_mut(popup_entity) else {
        return;
    };

    let scale = q_target
        .get(press.entity)
        .map(|t| t.scale_factor())
        .unwrap_or(1.0);
    let local_pos = transform
        .try_inverse()
        .unwrap()
        .transform_point2(press.pointer_location.position * scale / ui_scale.0);
    let pos = local_pos / node.size() + Vec2::splat(0.5);
    let pos = pos.clamp(Vec2::ZERO, Vec2::ONE);

    hsv.saturation = pos.x;
    hsv.value = 1.0 - pos.y;

    style.set("--color-picker-thumb-left", css_percent(pos.x * 100.0));
    style.set("--color-picker-thumb-top", css_percent(pos.y * 100.0));

    // Update the color control
    let new_color = hsv_to_color(hsv.hue, hsv.saturation, hsv.value);
    if let Ok(mut ctrl) = q_color.get_mut(owner.0) {
        ctrl.value = new_color;
    }
}

/// Observer: drag on the SV plane.
pub(crate) fn on_plane_drag_start(
    mut ev: On<Pointer<DragStart>>,
    q_inner: Query<&ChildOf, With<ColorPlaneInner>>,
    q_parent: Query<&ChildOf>,
    mut q_popup: Query<&mut ColorPlaneDragState, With<ColorPickerPopup>>,
) {
    let Ok(inner_parent) = q_inner.get(ev.entity) else {
        return;
    };
    ev.propagate(false);
    let plane_wrapper = inner_parent.parent();
    let Ok(wrapper_parent) = q_parent.get(plane_wrapper) else {
        return;
    };
    if let Ok(mut state) = q_popup.get_mut(wrapper_parent.parent()) {
        state.0 = true;
    }
}

/// Observer: drag on the SV plane — update position.
pub(crate) fn on_plane_drag(
    mut drag: On<Pointer<Drag>>,
    q_inner: Query<(&ComputedNode, &UiGlobalTransform, &ChildOf), With<ColorPlaneInner>>,
    q_parent: Query<&ChildOf>,
    mut q_popup: Query<
        (
            &ColorPlaneDragState,
            &mut ColorPickerHsv,
            &mut InlineStyle,
            &ColorPickerOwner,
        ),
        With<ColorPickerPopup>,
    >,
    mut q_color: Query<&mut ColorControl>,
    ui_scale: Res<UiScale>,
    q_target: Query<&ComputedUiRenderTargetInfo>,
) {
    let Ok((node, transform, inner_parent)) = q_inner.get(drag.entity) else {
        return;
    };
    drag.propagate(false);

    let plane_wrapper = inner_parent.parent();
    let Ok(wrapper_parent) = q_parent.get(plane_wrapper) else {
        return;
    };
    let popup_entity = wrapper_parent.parent();

    let Ok((drag_state, mut hsv, mut style, owner)) = q_popup.get_mut(popup_entity) else {
        return;
    };
    if !drag_state.0 {
        return;
    }

    let scale = q_target
        .get(drag.entity)
        .map(|t| t.scale_factor())
        .unwrap_or(1.0);
    let local_pos = transform
        .try_inverse()
        .unwrap()
        .transform_point2(drag.pointer_location.position * scale / ui_scale.0);
    let pos = local_pos / node.size() + Vec2::splat(0.5);
    let pos = pos.clamp(Vec2::ZERO, Vec2::ONE);

    hsv.saturation = pos.x;
    hsv.value = 1.0 - pos.y;

    style.set("--color-picker-thumb-left", css_percent(pos.x * 100.0));
    style.set("--color-picker-thumb-top", css_percent(pos.y * 100.0));

    let new_color = hsv_to_color(hsv.hue, hsv.saturation, hsv.value);
    if let Ok(mut ctrl) = q_color.get_mut(owner.0) {
        ctrl.value = new_color;
    }
}

/// Observer: drag end on the SV plane.
pub(crate) fn on_plane_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    q_inner: Query<&ChildOf, With<ColorPlaneInner>>,
    q_parent: Query<&ChildOf>,
    mut q_popup: Query<&mut ColorPlaneDragState, With<ColorPickerPopup>>,
) {
    let Ok(inner_parent) = q_inner.get(ev.entity) else {
        return;
    };
    ev.propagate(false);
    let plane_wrapper = inner_parent.parent();
    let Ok(wrapper_parent) = q_parent.get(plane_wrapper) else {
        return;
    };
    if let Ok(mut state) = q_popup.get_mut(wrapper_parent.parent()) {
        state.0 = false;
    }
}

/// Observer: drag cancel on the SV plane.
pub(crate) fn on_plane_drag_cancel(
    ev: On<Pointer<Cancel>>,
    q_inner: Query<&ChildOf, With<ColorPlaneInner>>,
    q_parent: Query<&ChildOf>,
    mut q_popup: Query<&mut ColorPlaneDragState, With<ColorPickerPopup>>,
) {
    let Ok(inner_parent) = q_inner.get(ev.entity) else {
        return;
    };
    let plane_wrapper = inner_parent.parent();
    let Ok(wrapper_parent) = q_parent.get(plane_wrapper) else {
        return;
    };
    if let Ok(mut state) = q_popup.get_mut(wrapper_parent.parent()) {
        state.0 = false;
    }
}

/// System: sync hue slider value → update HSV, shader material, and color.
pub(crate) fn sync_hue_slider(
    q_sliders: Query<(&SliderValue, &ChildOf), (With<HueSliderMarker>, Changed<SliderValue>)>,
    mut q_popup: Query<
        (&mut ColorPickerHsv, &mut InlineStyle, &ColorPickerOwner),
        With<ColorPickerPopup>,
    >,
    mut q_color: Query<&mut ColorControl>,
    q_children: Query<&Children>,
    q_material_node: Query<&MaterialNode<HsvPlaneMaterial>>,
    mut r_materials: ResMut<Assets<HsvPlaneMaterial>>,
) {
    for (slider_val, slider_parent) in &q_sliders {
        let popup_entity = slider_parent.parent();
        let Ok((mut hsv, mut style, owner)) = q_popup.get_mut(popup_entity) else {
            continue;
        };

        let new_hue = slider_val.0 * 360.0;
        hsv.hue = new_hue;

        style.set("--hue-slider-thumb-left", css_percent(slider_val.0 * 100.0));

        // Update the shader material
        for desc in q_children.iter_descendants(popup_entity) {
            if let Ok(mat_node) = q_material_node.get(desc) {
                if let Some(mat) = r_materials.get_mut(mat_node.id()) {
                    mat.hue = new_hue;
                }
                break;
            }
        }

        // Update color control
        let new_color = hsv_to_color(hsv.hue, hsv.saturation, hsv.value);
        if let Ok(mut ctrl) = q_color.get_mut(owner.0) {
            ctrl.value = new_color;
        }
    }
}

/// System: update hex text in picker when color changes.
pub(crate) fn update_picker_hex_text(
    q_popups: Query<(&ColorPickerOwner, Entity), With<ColorPickerPopup>>,
    q_colors: Query<&ColorControl, Changed<ColorControl>>,
    q_children: Query<&Children>,
    mut q_text: Query<&mut Text, With<ColorPickerHexText>>,
) {
    for (owner, popup_entity) in &q_popups {
        let Ok(ctrl) = q_colors.get(owner.0) else {
            continue;
        };
        let hex = color_to_hex(ctrl.value);
        for desc in q_children.iter_descendants(popup_entity) {
            if let Ok(mut text) = q_text.get_mut(desc) {
                text.0 = hex.clone();
                break;
            }
        }
    }
}

/// System: close color picker on Escape key.
pub(crate) fn close_color_picker_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    q_popups: Query<&ColorPickerOwner, With<ColorPickerPopup>>,
    mut q_open: Query<&mut ColorPickerOpen, With<ColorControlMarker>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for owner in &q_popups {
            if let Ok(mut open) = q_open.get_mut(owner.0) {
                open.0 = false;
            }
        }
    }
}

/// System: close color picker when clicking outside the popup and swatch.
pub(crate) fn close_color_picker_on_click_outside(
    mouse: Res<ButtonInput<MouseButton>>,
    q_popups: Query<(Entity, &ColorPickerOwner), With<ColorPickerPopup>>,
    q_popup_interaction: Query<&Interaction, With<ColorPickerPopup>>,
    q_swatch_interaction: Query<&Interaction, With<ColorSwatch>>,
    q_children: Query<&Children>,
    q_any_interaction: Query<&Interaction>,
    mut q_open: Query<&mut ColorPickerOpen, With<ColorControlMarker>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    for (popup_entity, owner) in &q_popups {
        // Check if click is on the popup or any descendant
        let popup_clicked = q_popup_interaction
            .get(popup_entity)
            .is_ok_and(|i| *i == Interaction::Pressed);

        let any_descendant_clicked = q_children.iter_descendants(popup_entity).any(|desc| {
            q_any_interaction
                .get(desc)
                .is_ok_and(|i| *i == Interaction::Pressed)
        });

        // Check if click is on any swatch
        let swatch_clicked = q_swatch_interaction
            .iter()
            .any(|i| *i == Interaction::Pressed);

        if !popup_clicked && !any_descendant_clicked && !swatch_clicked {
            if let Ok(mut open) = q_open.get_mut(owner.0) {
                open.0 = false;
            }
        }
    }
}
