use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use vismut_core::{
    live_graph::{LiveGraph, NodeState},
    node::SlotType,
    node_graph::NodeId,
};

#[derive(Component)]
pub struct LiveGraphComponent(pub Arc<RwLock<LiveGraph>>);

#[derive(Component, Default)]
pub struct NodeIdComponent(pub NodeId);

#[derive(Component, Default)]
pub struct NodeStateComponent(pub NodeState);

#[derive(Component, Default)]
pub struct SlotTypeComponent(pub SlotType);
