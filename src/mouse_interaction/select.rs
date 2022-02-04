use crate::{
    shared::NodeIdComponent,
    undo::{prelude::*, undo_command_manager::BoxUndoCommand, UndoCommand, UndoCommandType},
};
use bevy::prelude::*;
use vismut_core::node_graph::NodeId;

use super::active::MakeNodeNotActive;

#[derive(Component, Default)]
pub(crate) struct Selected;

fn select_node(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), Without<Selected>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).insert(Selected);
    } else {
        warn!("failed to select a node");
    }
}

fn deselect_node(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), With<Selected>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).remove::<Selected>();
    } else {
        warn!("failed to deselect a node");
    }
}

#[derive(Debug)]
pub struct ReplaceSelection(pub Vec<NodeId>);
impl UndoCommand for ReplaceSelection {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_node_id = world.query::<(&NodeIdComponent, Option<&Selected>)>();

        for (node_id, selected) in q_node_id.iter(world) {
            let in_new_selection = self.0.contains(&node_id.0);

            if selected.is_none() && in_new_selection {
                undo_command_manager
                    .commands
                    .push_front(Box::new(SelectNodeOnly(node_id.0)));
            } else if selected.is_some() && !in_new_selection {
                undo_command_manager
                    .commands
                    .push_front(Box::new(DeselectNodeOnly(node_id.0)));
            }
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("command is never put on undo stack");
    }
}

//
// Selecting
//

#[derive(Copy, Clone, Debug)]
struct SelectNodeOnly(NodeId);
impl UndoCommand for SelectNodeOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        select_node(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        deselect_node(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SelectNode(pub NodeId);
impl UndoCommand for SelectNode {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_node_id = world.query_filtered::<&NodeIdComponent, Without<Selected>>();

        if q_node_id.iter(world).any(|node_id| node_id.0 == self.0) {
            undo_command_manager.push_front(Box::new(SelectNodeOnly(self.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack");
    }
}

//
// Deselecting
//

#[derive(Copy, Clone, Debug)]
struct DeselectNodeOnly(NodeId);
impl UndoCommand for DeselectNodeOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        deselect_node(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        select_node(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeselectNode(pub NodeId);
impl UndoCommand for DeselectNode {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_node_id = world.query_filtered::<&NodeIdComponent, With<Selected>>();

        if q_node_id.iter(world).any(|node_id| node_id.0 == self.0) {
            let undo_batch: Vec<BoxUndoCommand> = vec![
                Box::new(MakeNodeNotActive(self.0)),
                Box::new(DeselectNodeOnly(self.0)),
            ];
            undo_command_manager.push_front_vec(undo_batch);
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeselectAll;
impl UndoCommand for DeselectAll {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_selected = world.query_filtered::<&NodeIdComponent, With<Selected>>();

        for node_id in q_selected.iter(world) {
            undo_command_manager
                .commands
                .push_front(Box::new(DeselectNodeOnly(node_id.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("command is never put on undo stack");
    }
}
