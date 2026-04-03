use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;

const SCROLL_LINE_HEIGHT: f32 = 21.0;

/// Entity event for UI scroll — propagates up the hierarchy.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub(crate) struct UiScroll {
    pub entity: Entity,
    pub delta: Vec2,
}

/// System: read MouseWheel input and trigger UiScroll on hovered entities.
pub(crate) fn send_ui_scroll_events(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<bevy::picking::hover::HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= SCROLL_LINE_HEIGHT;
        }

        // Ctrl+Wheel swaps X/Y for horizontal scroll
        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        // Trigger on all entities hovered by the default pointer
        if let Some(pointer_map) = hover_map.get(&PointerId::Mouse) {
            for entity in pointer_map.keys().copied() {
                commands.trigger(UiScroll { entity, delta });
            }
        }
    }
}

/// Observer: handle UiScroll by updating ScrollPosition.
pub(crate) fn handle_ui_scroll(
    mut scroll: On<UiScroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
    let delta = &mut scroll.delta;

    let is_scrollable_x = node.overflow.x == OverflowAxis::Scroll;
    let is_scrollable_y = node.overflow.y == OverflowAxis::Scroll;

    // Y-axis scrolling
    if is_scrollable_y && delta.y != 0.0 {
        let at_limit = if delta.y > 0.0 {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.0
        };

        if !at_limit {
            scroll_position.y = (scroll_position.y + delta.y).clamp(0.0, max_offset.y.max(0.0));
            delta.y = 0.0;
        }
    }

    // X-axis scrolling
    if is_scrollable_x && delta.x != 0.0 {
        let at_limit = if delta.x > 0.0 {
            scroll_position.x >= max_offset.x
        } else {
            scroll_position.x <= 0.0
        };

        if !at_limit {
            scroll_position.x = (scroll_position.x + delta.x).clamp(0.0, max_offset.x.max(0.0));
            delta.x = 0.0;
        }
    }

    // If this node is scrollable on any axis, consume the entire event so it
    // doesn't leak to a parent scroll container on the other axis.
    // (e.g. horizontal tab bar eating a vertical wheel gesture that would
    //  otherwise scroll the pane body behind it)
    if is_scrollable_x || is_scrollable_y {
        scroll.propagate(false);
    }
}
