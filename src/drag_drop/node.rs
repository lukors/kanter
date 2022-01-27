use std::fmt::Debug;

use crate::{
    shared::NodeIdComponent, stretch_between, undo::prelude::*, Cursor, Edge as GuiEdge, Selected,
    Slot, ToolState,
};
use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

use super::Dragged;

#[derive(Clone, Debug)]
pub struct MoveNodeUndo {
    pub node_id: NodeId,
    pub from: Vec2,
    pub to: Vec2,
}

impl UndoCommand for MoveNodeUndo {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query::<(&NodeIdComponent, &mut Transform)>();
        if let Some((_, mut transform)) = query
            .iter_mut(world)
            .find(|(node_id, _)| node_id.0 == self.node_id)
        {
            transform.translation.x = self.to.x;
            transform.translation.y = self.to.y;
            update_node_gui_edges(world, self.node_id);
        }
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query::<(&NodeIdComponent, &mut Transform)>();
        if let Some((_, mut transform)) = query
            .iter_mut(world)
            .find(|(node_id, _)| node_id.0 == self.node_id)
        {
            transform.translation.x = self.from.x;
            transform.translation.y = self.from.y;
            update_node_gui_edges(world, self.node_id);
        }
    }
}

/// Grab all selected nodes.
pub(crate) fn grab_tool_node_setup(
    mut tool_state: ResMut<State<ToolState>>,
    mut commands: Commands,
    q_selected_nodes: Query<(Entity, &GlobalTransform), (With<NodeIdComponent>, With<Selected>)>,
) {
    if q_selected_nodes.iter().count() == 0 {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for (entity, gtransform) in q_selected_nodes.iter() {
        commands.entity(entity).insert(Dragged {
            start: gtransform.translation.truncate(),
        });
    }
}

/// Updates all dragged nodes.
pub(crate) fn drag_node_update(
    mut commands: Commands,
    mut q_dragged_node: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (Added<Dragged>, With<NodeIdComponent>, Without<Slot>),
    >,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
) {
    if let Ok((cursor_e, cursor_transform)) = q_cursor.get_single() {
        for (entity, mut transform, global_transform) in q_dragged_node.iter_mut() {
            commands.entity(cursor_e).push_children(&[entity]);

            let global_pos = global_transform.translation - cursor_transform.translation;
            transform.translation.x = global_pos.x;
            transform.translation.y = global_pos.y;
        }
    }
}

fn update_node_gui_edges(world: &mut World, node_id: NodeId) {
    let node_transform = *world
        .query::<(&NodeIdComponent, &Transform)>()
        .iter(world)
        .find(|(node_id_iter, _)| node_id_iter.0 == node_id)
        .map(|(_, transform)| transform)
        .unwrap();
    let slots = world
        .query::<(&Slot, &Transform)>()
        .iter(world)
        .filter(|(slot, _)| slot.node_id == node_id)
        .map(|(slot, transform)| (*slot, *transform))
        .collect::<Vec<(Slot, Transform)>>();

    let mut q_edge = world.query::<(&mut Sprite, &mut Transform, &mut GuiEdge)>();

    for (mut sprite, mut edge_t, mut edge) in q_edge.iter_mut(world).filter(|(_, _, edge)| {
        edge.input_slot.node_id == node_id || edge.output_slot.node_id == node_id
    }) {
        for (slot, slot_t) in slots.iter() {
            if slot.node_id == edge.output_slot.node_id
                && slot.slot_id == edge.output_slot.slot_id
                && slot.side == edge.output_slot.side
            {
                edge.start = (node_transform.translation + slot_t.translation).truncate();
            } else if slot.node_id == edge.input_slot.node_id
                && slot.slot_id == edge.input_slot.slot_id
                && slot.side == edge.input_slot.side
            {
                edge.end = (node_transform.translation + slot_t.translation).truncate();
            }
        }

        stretch_between(&mut sprite, &mut edge_t, edge.start, edge.end);
    }
}
