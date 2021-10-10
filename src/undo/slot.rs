// use std::sync::{Arc, RwLock};

// use crate::sync_graph::{stretch_between, Edge as GuiEdge, Slot};

// use super::{prelude::*, AddRemove};
// use anyhow::{anyhow, bail, Result};
// use bevy::prelude::*;
// use kanter_core::{
//     edge::Edge,
//     live_graph::LiveGraph,
//     node::Side,
//     node_graph::{NodeId, SlotId},
// };

// #[derive(Clone, Copy, Debug)]
// pub struct DisconnectSlot(pub Slot);
// impl UndoCommand for DisconnectSlot {
//     fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
//         if let Some(source_slot) = source_slot {
//             match live_graph.write().unwrap().connected_edges(
//                 source_slot.0.node_id,
//                 source_slot.0.side,
//                 source_slot.0.slot_id,
//             ) {
//                 Ok(edges) => {
//                     for edge in edges {
//                         undo_command_manager.push(Box::new(RemoveEdge(edge)));
//                         undo_command_manager.push(Box::new(Checkpoint));
//                         // info!(
//                         //     "Removing edge from {:?} {:?} to {:?} {:?}",
//                         //     edge.output_id, edge.output_slot, edge.input_id, edge.input_slot
//                         // );
//                     }
//                 }
//                 Err(e) => {
//                     error!(
//                         "Failed to disconnect slot: NodeId({}), Side({:?}), SlotId({}): {}",
//                         source_slot.0.node_id, source_slot.0.side, source_slot.0.slot_id, e
//                     );
//                 }
//             }
//         }
//     }

//     fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
//         self.0.add(world);
//     }
// }
