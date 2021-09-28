use std::sync::{Arc, RwLock};

use super::{
    undo_command_manager::{UndoCommand, UndoCommandManager},
    AddRemove,
};
use bevy::prelude::{error, World};
use kanter_core::{edge::Edge, live_graph::LiveGraph};

impl AddRemove for Edge {
    fn add(&self, world: &mut World) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if live_graph
                    .connect(
                        self.output_id(),
                        self.input_id(),
                        self.output_slot(),
                        self.input_slot(),
                    )
                    .is_err()
                {
                    error!("Couldn't add the edge");
                }
            }
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
pub struct RemoveEdge(Edge);
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
