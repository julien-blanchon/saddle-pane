//! UX systems: tooltip rendering.

use bevy::prelude::*;

use crate::controls::PaneControlMeta;

/// Marker component for tooltip popup entities.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneTooltip;

/// Marker for the tooltip text child.
#[derive(Component, Clone, Debug, Default)]
pub(crate) struct PaneTooltipText;

/// Tracks which control entity the tooltip is currently showing for.
#[derive(Resource, Default, Debug)]
pub(crate) struct TooltipState {
    pub active_control: Option<Entity>,
    pub tooltip_entity: Option<Entity>,
}

impl TooltipState {
    /// Despawn the active tooltip and clear state.
    fn dismiss(&mut self, commands: &mut Commands) {
        if let Some(old) = self.tooltip_entity.take() {
            commands.entity(old).despawn();
        }
        self.active_control = None;
    }
}

/// System: show tooltip on hover for controls that have one.
pub(crate) fn show_tooltip(
    q_interactions: Query<
        (
            Entity,
            &PaneControlMeta,
            &Interaction,
            &GlobalTransform,
            &ComputedNode,
        ),
        Changed<Interaction>,
    >,
    mut tooltip_state: ResMut<TooltipState>,
    mut commands: Commands,
) {
    for (entity, meta, interaction, global_transform, computed) in &q_interactions {
        match interaction {
            Interaction::Hovered => {
                let Some(ref tooltip_text) = meta.tooltip else {
                    continue;
                };
                if tooltip_state.active_control == Some(entity) {
                    continue;
                }

                // Despawn existing tooltip before spawning new one
                tooltip_state.dismiss(&mut commands);

                // Calculate position: below the control row, aligned left
                let transform = global_transform.translation();
                let size = computed.size();
                let inv_scale = computed.inverse_scale_factor;
                let left = (transform.x - size.x / 2.0) * inv_scale;
                let top = (transform.y + size.y / 2.0) * inv_scale + 2.0;

                let tooltip_entity = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(left),
                            top: Val::Px(top),
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                            max_width: Val::Px(200.0),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.12, 0.12, 0.14, 0.95)),
                        GlobalZIndex(950),
                        PaneTooltip,
                    ))
                    .with_children(|tooltip| {
                        tooltip.spawn((
                            Text::new(tooltip_text),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.75, 0.75, 0.77, 1.0)),
                            PaneTooltipText,
                        ));
                    })
                    .id();

                tooltip_state.active_control = Some(entity);
                tooltip_state.tooltip_entity = Some(tooltip_entity);
            }
            Interaction::None | Interaction::Pressed => {
                if tooltip_state.active_control == Some(entity) {
                    tooltip_state.dismiss(&mut commands);
                }
            }
        }
    }
}

/// System: clean up tooltip if the active control was despawned.
pub(crate) fn cleanup_orphaned_tooltips(
    mut tooltip_state: ResMut<TooltipState>,
    q_exists: Query<(), With<PaneControlMeta>>,
    mut commands: Commands,
) {
    if let Some(control) = tooltip_state.active_control {
        if q_exists.get(control).is_err() {
            tooltip_state.dismiss(&mut commands);
        }
    }
}
