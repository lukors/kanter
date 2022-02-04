use std::fmt::Debug;

use crate::{
    shared::NodeIdComponent, stretch_between, undo::prelude::*, Cursor, Edge as GuiEdge, Selected,
    Slot, ToolState,
};
use bevy::prelude::*;
use vismut_core::node_graph::NodeId;

use super::{Dragged, Dropped};

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
pub(crate) fn grab_node_setup(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    mut q_selected_nodes: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (With<NodeIdComponent>, With<Selected>),
    >,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
) {
    let (cursor_e, cursor_transform) = q_cursor.single();
    let mut any_nodes = false;

    for (entity, mut transform, global_transform) in q_selected_nodes.iter_mut() {
        commands.entity(cursor_e).push_children(&[entity]);
        commands.entity(entity).insert(Dragged {
            start: global_transform.translation.truncate(),
        });

        let cursor_space = global_transform.translation - cursor_transform.translation;
        transform.translation.x = cursor_space.x;
        transform.translation.y = cursor_space.y;

        any_nodes = true;
    }

    if !any_nodes {
        tool_state.overwrite_replace(ToolState::None).unwrap();
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

/// Exit grab tool if mouse button is released.
pub(crate) fn grab_node_update(
    mut commands: Commands,
    mut undo_command_manager: ResMut<UndoCommandManager>,
    q_dragged: Query<(Entity, &NodeIdComponent, &Dragged, &GlobalTransform)>,
    mut tool_state: ResMut<State<ToolState>>,
    mut i_mouse_button: ResMut<Input<MouseButton>>,
) {
    if i_mouse_button.just_released(MouseButton::Left) {
        for (entity, node_id, dragged, gtransform) in q_dragged.iter() {
            let to = gtransform.translation.truncate();

            undo_command_manager.push(Box::new(MoveNodeUndo {
                node_id: node_id.0,
                from: dragged.start,
                to,
            }));

            commands.entity(entity).remove::<Parent>();
            commands.entity(entity).remove::<Dragged>();
        }

        undo_command_manager.push(Box::new(Checkpoint));
        tool_state.overwrite_replace(ToolState::None).unwrap();

        i_mouse_button.clear();
    }
}

pub(crate) fn grab_node_cleanup(
    mut commands: Commands,
    mut q_dragged: Query<
        (Entity, Option<&Dragged>, Option<&Dropped>, &mut Transform),
        With<NodeIdComponent>,
    >,
) {
    for (entity, dragged, dropped, mut transform) in q_dragged.iter_mut() {
        if dragged.is_some() || dropped.is_some() {
            commands.entity(entity).remove::<Dragged>();
            commands.entity(entity).remove::<Dropped>();
            commands.entity(entity).remove::<Parent>();

            if let Some(dragged) = dragged {
                transform.translation.x = dragged.start.x;
                transform.translation.y = dragged.start.y;
            }
        }
    }
}

pub(crate) fn grab_node_update_edge(
    q_node: Query<(&NodeIdComponent, &Transform), With<Dragged>>,
    q_slot: Query<(&Slot, &Transform)>,
    mut q_edge: Query<
        (&mut Sprite, &mut Transform, &mut GuiEdge),
        (Without<NodeIdComponent>, Without<Slot>, Without<Cursor>),
    >,
    q_cursor: Query<&Transform, With<Cursor>>,
) {
    let cursor_t = q_cursor.iter().next().unwrap().translation;

    for (node_id, node_t) in q_node.iter() {
        for (mut sprite, mut edge_t, mut edge) in q_edge.iter_mut().filter(|(_, _, edge)| {
            edge.input_slot.node_id == node_id.0 || edge.output_slot.node_id == node_id.0
        }) {
            for (slot, slot_t) in q_slot.iter().filter(|(slot, _)| slot.node_id == node_id.0) {
                if slot.node_id == edge.output_slot.node_id
                    && slot.slot_id == edge.output_slot.slot_id
                    && slot.side == edge.output_slot.side
                {
                    edge.start = (cursor_t + node_t.translation + slot_t.translation).truncate();
                } else if slot.node_id == edge.input_slot.node_id
                    && slot.slot_id == edge.input_slot.slot_id
                    && slot.side == edge.input_slot.side
                {
                    edge.end = (cursor_t + node_t.translation + slot_t.translation).truncate();
                }
            }

            stretch_between(&mut sprite, &mut edge_t, edge.start, edge.end);
        }
    }
}
