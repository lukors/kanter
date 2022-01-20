use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use kanter_core::{node_graph::NodeId, node::SlotType, live_graph::{NodeState, LiveGraph}};

#[derive(Component)]
pub struct LiveGraphComponent(pub Arc<RwLock<LiveGraph>>);

#[derive(Component)]
pub struct NodeIdComponent(pub NodeId);

#[derive(Component)]
pub struct NodeStateComponent(pub NodeState);

#[derive(Component)]
pub struct SlotTypeComponent(pub SlotType);