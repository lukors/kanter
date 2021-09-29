use std::sync::{Arc, RwLock};

use bevy::prelude::{error, World};
use kanter_core::{
    live_graph::{self, LiveGraph},
    node::Node,
};

use super::{
    edge::{AddEdge, RemoveEdge},
    prelude::*,
    AddRemove,
};

impl AddRemove for Node {
    fn add(&self, world: &mut World) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph.add_node_with_id(self.clone()).is_err() {
                    error!("Couldn't add node");
                }
            }
        }
    }

    fn remove(&self, world: &mut World) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph.remove_node(self.node_id).is_err() {
                    error!("Couldn't find the node to remove");
                }
            }
        }
    }
}

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
    undo_batch.push(Box::new(RemoveNode(node)));

    undo_batch
}

#[derive(Clone, Debug)]
pub struct AddNode(Node);
impl UndoCommand for AddNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.add(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.remove(world);
    }
}

#[derive(Clone, Debug)]
struct RemoveNode(Node);
impl UndoCommand for RemoveNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.remove(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.add(world);
    }
}
