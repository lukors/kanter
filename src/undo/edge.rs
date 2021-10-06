use std::sync::{Arc, RwLock};

use super::{prelude::*, AddRemove};
use anyhow::{anyhow, Result};
use bevy::prelude::{error, World};
use kanter_core::{
    edge::Edge,
    live_graph::LiveGraph,
    node::Side,
    node_graph::{NodeId, SlotId},
};

impl AddRemove for Edge {
    fn add(&self, world: &mut World) {
        if let Some(live_graph) = world.remove_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph
                    .connect(
                        self.output_id(),
                        self.input_id(),
                        self.output_slot(),
                        self.input_slot(),
                    )
                    .is_ok()
                {
                } else {
                    error!("Couldn't add the edge");
                }
            }
            world.insert_resource(live_graph);
        }
    }

    fn remove(&self, world: &mut World) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph.remove_edge(*self).is_err() {
                    error!("Couldn't find the edge to remove");
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RemoveEdge(pub Edge);
impl UndoCommand for RemoveEdge {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.remove(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.add(world);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AddEdge(Edge);
impl UndoCommand for AddEdge {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.add(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.remove(world);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConnectArbitrary {
    pub a_node: NodeId,
    pub a_side: Side,
    pub a_slot: SlotId,
    pub b_node: NodeId,
    pub b_side: Side,
    pub b_slot: SlotId,
}
impl UndoCommand for ConnectArbitrary {
    fn command_type(&self) -> super::UndoCommandType {
        super::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        if let Ok(edge) = self.connect(world) {
            undo_command_manager
                .undo_stack
                .push(Box::new(AddEdge(edge)));
        }
    }

    fn backward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack")
    }
}
impl ConnectArbitrary {
    fn connect(&self, world: &mut World) -> Result<Edge> {
        world
            .get_resource::<Arc<RwLock<LiveGraph>>>()
            .ok_or(anyhow!("could not get resource"))?
            .write()
            .map_err(|e| anyhow!("unable to get write lock: {}", e))?
            .connect_arbitrary(
                self.a_node,
                self.a_side,
                self.a_slot,
                self.b_node,
                self.b_side,
                self.b_slot,
            )
            .map_err(|e| anyhow!("could not create edge: {}", e))
    }
}
