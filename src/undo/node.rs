use bevy::prelude::*;
use kanter_core::node::Node;

use crate::{
    mouse_interaction::{active::MakeNothingActive, select::DeselectNode},
    sync_graph::{self, Edge},
};

use super::{edge::RemoveGuiEdge, prelude::*, undo_command_manager::BoxUndoCommand};

#[derive(Clone, Debug)]
pub struct AddNode {
    pub node: Node,
    pub translation: Vec2,
}
impl UndoCommand for AddNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        sync_graph::spawn_gui_node_2(world, self.node.clone(), self.translation);

        // self.node.add(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        sync_graph::remove_gui_node(world, self.node.node_id);
        // world.get_resource::<Arc<RwLock<LiveGraph>>>().unwrap().write().unwrap().remove_node(self.node.node_id).unwrap();
    }
}
impl AddNode {
    pub fn new(node: Node, translation: Vec2) -> Self {
        Self { node, translation }
    }
}

/// Removes only the `Node`, and none of the connected `Edge`s. You should almost always use
/// `RemoveNode` instead, which removes connected edges too.
#[derive(Clone, Debug)]
pub struct RemoveNodeOnly {
    pub node: Node,
    pub translation: Vec2,
}
impl UndoCommand for RemoveNodeOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        sync_graph::remove_gui_node(world, self.node.node_id);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        sync_graph::spawn_gui_node_2(world, self.node.clone(), self.translation);
    }
}
impl RemoveNodeOnly {
    pub fn new(node: Node, translation: Vec2) -> Self {
        Self { node, translation }
    }
}

/// Removes the `Node` and all connected `Edge`s.
#[derive(Clone, Debug)]
pub struct RemoveNode {
    pub node: Node,
    pub translation: Vec2,
}
impl UndoCommand for RemoveNode {
    fn command_type(&self) -> super::UndoCommandType {
        super::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut commands: Vec<BoxUndoCommand> = Vec::new();

        let mut q_edge = world.query::<&Edge>();

        for edge in q_edge.iter(world).filter(|edge| {
            edge.input_slot.node_id == self.node.node_id
                || edge.output_slot.node_id == self.node.node_id
        }) {
            commands.push(Box::new(RemoveGuiEdge(*edge)));
        }

        commands.push(Box::new(DeselectNode(self.node.node_id)));
        commands.push(Box::new(MakeNothingActive));
        commands.push(Box::new(RemoveNodeOnly {
            node: self.node.clone(),
            translation: self.translation,
        }));

        undo_command_manager.push_front_vec(commands);
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack, so this can not be reached");
    }
}
impl RemoveNode {
    pub fn new(node: Node, translation: Vec2) -> Self {
        Self { node, translation }
    }
}
