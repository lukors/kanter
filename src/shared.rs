use bevy::prelude::*;
use kanter_core::{node_graph::NodeId, node::SlotType};

#[derive(Component)]
pub struct NodeIdComponent(pub NodeId);

#[derive(Component)]
pub struct SlotTypeComponent(pub SlotType);