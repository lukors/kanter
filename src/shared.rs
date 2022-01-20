use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

#[derive(Component)]
pub struct NodeIdComponent(pub NodeId);