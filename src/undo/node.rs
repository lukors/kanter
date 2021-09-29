use std::sync::{Arc, RwLock};

use bevy::prelude::{error, World};
use kanter_core::{live_graph::LiveGraph, node::Node};

use super::{prelude::*, AddRemove};

impl AddRemove for Node {
    fn add(&self, world: &mut World) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph.add_node(self.clone()).is_err() {
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

#[derive(Clone, Debug)]
struct AddNode(Node);
impl UndoCommand for AddNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        // Todo: Create an undo queue and add it to the undo_command_manager
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
