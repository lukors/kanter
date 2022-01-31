use anyhow::{anyhow, Result};
use std::fmt::Debug;

use crate::{
    control_pressed,
    hoverable::box_contains_point,
    mouse_interaction::Selected,
    scan_code_input::ScanCodeInput,
    stretch_between,
    undo::{
        edge::{AddEdge, RemoveGuiEdge},
        prelude::*,
        undo_command_manager::Checkpoint,
    },
    Cursor, Edge as GuiEdge, Slot, ToolState,
};
use bevy::prelude::*;
use kanter_core::{edge::Edge, node::Side};

use super::Dragged;

#[derive(Component, Copy, Clone, Debug)]
pub struct SourceSlot(Slot);

#[derive(Component)]
pub struct GrabbedEdge {
    start: Vec2,
    slot: Slot,
}

/// Grab all selected slots.
pub(crate) fn grab_tool_slot_setup(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    q_selected_slots: Query<(Entity, &GlobalTransform, &Slot), With<Selected>>,
    q_slot: Query<(&GlobalTransform, &Slot)>,
    mut q_edge: Query<(&mut Visibility, &GuiEdge)>,
    scan_code_input: Res<ScanCodeInput>,
) {
    let line_sprite_bundle = SpriteBundle {
        sprite: Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::new(5.0, 5.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    if q_selected_slots.iter().count() == 0 {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for (entity, global_transform, slot) in q_selected_slots.iter() {
        commands.entity(entity).insert(Dragged {
            start: global_transform.translation.truncate(),
        });

        if control_pressed(&scan_code_input) {
            match slot.side {
                Side::Output => {
                    for (mut edge_visible, edge) in q_edge
                        .iter_mut()
                        .filter(|(_, edge)| edge.output_slot == *slot)
                    {
                        edge_visible.is_visible = false;

                        if let Some((input_slot_gtransform, input_slot)) =
                            q_slot.iter().find(|(_, slot)| {
                                slot.node_id == edge.input_slot.node_id
                                    && slot.slot_id == edge.input_slot.slot_id
                                    && slot.side == Side::Input
                            })
                        {
                            commands
                                .spawn_bundle(line_sprite_bundle.clone())
                                .insert(GrabbedEdge {
                                    start: input_slot_gtransform.translation.truncate(),
                                    slot: *input_slot,
                                })
                                .insert(SourceSlot(*slot));
                        }
                    }
                }
                Side::Input => {
                    if let Some((mut edge_visible, edge)) =
                        q_edge.iter_mut().find(|(_, edge)| edge.input_slot == *slot)
                    {
                        edge_visible.is_visible = false;

                        if let Some((output_slot_gtransform, output_slot)) =
                            q_slot.iter().find(|(_, slot)| {
                                slot.node_id == edge.output_slot.node_id
                                    && slot.slot_id == edge.output_slot.slot_id
                                    && slot.side == Side::Output
                            })
                        {
                            commands
                                .spawn_bundle(line_sprite_bundle.clone())
                                .insert(GrabbedEdge {
                                    start: output_slot_gtransform.translation.truncate(),
                                    slot: *output_slot,
                                })
                                .insert(SourceSlot(*slot));
                        }
                    }
                }
            }
        } else {
            commands
                .spawn_bundle(line_sprite_bundle.clone())
                .insert(GrabbedEdge {
                    start: global_transform.translation.truncate(),
                    slot: *slot,
                });
        }
    }
}

/// When an edge is dropped, this system updates the node graph based on where its dropped, and
/// removes the edges.
pub(crate) fn grab_edge_update(
    mut undo_command_manager: ResMut<UndoCommandManager>,
    mut tool_state: ResMut<State<ToolState>>,
    mut i_mouse_button: ResMut<Input<MouseButton>>,
    q_slot: Query<(&GlobalTransform, &Sprite, &Slot)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
    mut q_grabbed_edge: Query<
        (
            &mut Transform,
            &mut Sprite,
            &GrabbedEdge,
            Option<&SourceSlot>,
        ),
        Without<Slot>,
    >,
    q_edge: Query<&GuiEdge, Without<GrabbedEdge>>,
) {
    let cursor_t = q_cursor.iter().next().unwrap();

    if i_mouse_button.just_released(MouseButton::Left) {
        i_mouse_button.clear();
        
        let mut new_edges = Vec::new();

        'outer: for (_, _, grabbed_edge, source_slot) in q_grabbed_edge.iter() {
            for (slot_t, slot_sprite, slot) in q_slot.iter() {
                if let Some(size) = slot_sprite.custom_size {
                    if box_contains_point(
                        slot_t.translation.truncate(),
                        size,
                        cursor_t.translation.truncate(),
                    ) {
                        if let Some(source_slot) = source_slot {
                            if source_slot.0 != *slot {
                                if let Ok(add_edge) = connect_arbitrary(*slot, grabbed_edge.slot) {
                                    new_edges.push(Box::new(add_edge));
                                }
                                for edge in q_edge.iter() {
                                    if (edge.input_slot == source_slot.0
                                        && edge.output_slot == grabbed_edge.slot)
                                        || (edge.output_slot == source_slot.0
                                            && edge.input_slot == grabbed_edge.slot)
                                    {
                                        undo_command_manager.push(Box::new(RemoveGuiEdge(*edge)));
                                    }
                                }
                            }
                        } else if let Ok(add_edge) = connect_arbitrary(*slot, grabbed_edge.slot) {
                            new_edges.push(Box::new(add_edge));
                        }

                        continue 'outer;
                    }
                }
            }

            if let Some(source_slot) = source_slot {
                for edge in q_edge.iter() {
                    if (edge.input_slot == source_slot.0 && edge.output_slot == grabbed_edge.slot)
                        || (edge.output_slot == source_slot.0
                            && edge.input_slot == grabbed_edge.slot)
                    {
                        undo_command_manager.push(Box::new(RemoveGuiEdge(*edge)));
                    }
                }
            }
        }

        for new_edge in new_edges {
            undo_command_manager.push(new_edge);
        }
        undo_command_manager.push(Box::new(Checkpoint));
        tool_state.overwrite_replace(ToolState::None).unwrap();
    } else {
        // Update grabbed edge
        for (mut edge_t, mut sprite, edge, _) in q_grabbed_edge.iter_mut() {
            stretch_between(
                &mut sprite,
                &mut edge_t,
                edge.start,
                cursor_t.translation.truncate(),
            );
        }
    }
}

fn connect_arbitrary(slot_a: Slot, slot_b: Slot) -> Result<AddEdge> {
    if let Ok(edge) = Edge::from_arbitrary(
        slot_a.node_id,
        slot_a.side,
        slot_a.slot_id,
        slot_b.node_id,
        slot_b.side,
        slot_b.slot_id,
    ) {
        Ok(AddEdge(edge))
    } else {
        Err(anyhow!("could not connect slot"))
    }
}

/// Drops all grabbed entities.
pub(crate) fn grab_edge_cleanup(
    mut commands: Commands,
    q_grabbed_edge: Query<Entity, With<GrabbedEdge>>,
    q_dragged: Query<Entity, With<Dragged>>,
    mut q_edge: Query<&mut Visibility, (With<GuiEdge>, Without<Dragged>)>,
) {
    for entity in q_grabbed_edge.iter() {
        commands.entity(entity).despawn_recursive();
    }

    for entity in q_dragged.iter() {
        commands.entity(entity).remove::<Dragged>();
    }

    for mut visibility in q_edge.iter_mut() {
        *visibility = Visibility { is_visible: true };
    }
}
