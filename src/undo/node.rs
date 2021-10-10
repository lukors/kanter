use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use kanter_core::{live_graph::LiveGraph, node::Node};

use crate::sync_graph;

use super::{edge::RemoveEdge, prelude::*};

// impl AddRemove for Node {
//     fn add(&self, world: &mut World) -> Entity {
//         sync_graph::spawn_gui_node_2(world, self.clone(), )
//         // world.get_resource::<Arc<RwLock<LiveGraph>>>().unwrap().write().unwrap().add_node_with_id(self.clone()).unwrap();
//         // if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
//         //     if let Ok(mut live_graph) = live_graph.write() {
//         //         if live_graph.add_node_with_id(self.clone()).is_err() {
//         //             error!("Couldn't add node");
//         //         }
//         //     }
//         // }
//         todo!();
//     }

//     fn remove(&self, world: &mut World) {
//         if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
//             if let Ok(mut live_graph) = live_graph.write() {
//                 if live_graph.remove_node(self.node_id).is_err() {
//                     error!("Couldn't find the node to remove");
//                 }
//             }
//         }
//     }
// }

pub fn remove_node(live_graph: &Arc<RwLock<LiveGraph>>, node: Node) -> Vec<Box<dyn UndoCommand>> {
    let mut undo_batch: Vec<Box<dyn UndoCommand>> = Vec::new();

    for edge in live_graph
        .read()
        .unwrap()
        .edges()
        .iter()
        .filter(|edge| edge.input_id() == node.node_id || edge.output_id() == node.node_id)
    {
        undo_batch.push(Box::new(RemoveEdge(*edge)));
    }
    undo_batch.push(Box::new(RemoveNode::new(node, Vec2::ZERO)));

    undo_batch
}

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

#[derive(Clone, Debug)]
pub struct RemoveNode {
    pub node: Node,
    pub translation: Vec2,
}
impl UndoCommand for RemoveNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        sync_graph::remove_gui_node(world, self.node.node_id);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        sync_graph::spawn_gui_node_2(world, self.node.clone(), self.translation);
    }
}
impl RemoveNode {
    pub fn new(node: Node, translation: Vec2) -> Self {
        Self { node, translation }
    }
}
