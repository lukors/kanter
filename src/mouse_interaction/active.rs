use crate::{
    mouse_interaction::select::SelectNode,
    shared::NodeIdComponent,
    undo::{prelude::*, UndoCommand, UndoCommandType},
};
use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

#[derive(Component, Default)]
pub(crate) struct Active;

fn make_node_active(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), Without<Active>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).insert(Active);
    } else {
        warn!("failed to make a node active");
    }
}

fn make_node_not_active(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), With<Active>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).remove::<Active>();
    } else {
        warn!("failed to make a node not active");
    }
}

//
// Making nodes active
//

#[derive(Copy, Clone, Debug)]
struct MakeNodeActiveOnly(pub NodeId);
impl UndoCommand for MakeNodeActiveOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_active(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_not_active(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNodeActive(pub NodeId);
impl UndoCommand for MakeNodeActive {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_active_node_id = world.query_filtered::<&NodeIdComponent, With<Active>>();
        assert!(
            q_active_node_id.iter(world).count() < 2,
            "there is more than one active node"
        );

        if let Some(active_node_id) = q_active_node_id.iter(world).next() {
            if active_node_id.0 != self.0 {
                undo_command_manager.push_front(Box::new(MakeNodeNotActiveOnly(active_node_id.0)));
                undo_command_manager.push_front(Box::new(SelectNode(self.0)));
                undo_command_manager.push_front(Box::new(MakeNodeActiveOnly(self.0)));
            }
        } else {
            undo_command_manager.push_front(Box::new(SelectNode(self.0)));
            undo_command_manager.push_front(Box::new(MakeNodeActiveOnly(self.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack");
    }
}

//
// Making nodes not active
//

#[derive(Copy, Clone, Debug)]
struct MakeNodeNotActiveOnly(pub NodeId);
impl UndoCommand for MakeNodeNotActiveOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_not_active(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_active(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNodeNotActive(pub NodeId);
impl UndoCommand for MakeNodeNotActive {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_active_node_id = world.query_filtered::<&NodeIdComponent, With<Active>>();
        assert!(
            q_active_node_id.iter(world).count() < 2,
            "there is more than one active node"
        );

        if let Some(active_node_id) = q_active_node_id.iter(world).next() {
            if active_node_id.0 == self.0 {
                undo_command_manager.push_front(Box::new(MakeNodeNotActiveOnly(self.0)));
            } else {
                warn!("tried making a not active node not active");
            }
        } else {
            warn!("could not find an active node to make not active");
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNothingActive;
impl UndoCommand for MakeNothingActive {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_active_node_id = world.query_filtered::<&NodeIdComponent, With<Active>>();
        assert!(
            q_active_node_id.iter(world).count() < 2,
            "there is more than one active node"
        );

        if let Some(node_id) = q_active_node_id.iter(world).next() {
            undo_command_manager.push_front(Box::new(MakeNodeNotActive(node_id.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this undo command is never put on the undo stack");
    }
}
