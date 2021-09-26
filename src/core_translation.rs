use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use anyhow::{bail, Result};
use kanter_core::{
    live_graph::LiveGraph,
    node::{node_type::NodeType, ResizeFilter, ResizePolicy},
    node_graph::NodeId,
    slot_data::ChannelPixel,
};

use crate::undo_command_manager::{UndoCommand, UndoCommandManager};

pub trait Translator<DataType>: Debug {
    fn get(&self, live_graph: &LiveGraph) -> Result<DataType>;
    fn set(&self, live_graph: &mut LiveGraph, value: DataType) -> Result<()>;
}

impl Translator<NodeType> for NodeId {
    fn get(&self, live_graph: &LiveGraph) -> Result<NodeType> {
        Ok(live_graph.node(*self)?.node_type)
    }

    fn set(&self, live_graph: &mut LiveGraph, value: NodeType) -> Result<()> {
        live_graph.node_mut(*self)?.node_type = value;
        Ok(())
    }
}

impl Translator<ResizePolicy> for NodeId {
    fn get(&self, live_graph: &LiveGraph) -> Result<ResizePolicy> {
        Ok(live_graph.node(*self)?.resize_policy)
    }

    fn set(&self, live_graph: &mut LiveGraph, value: ResizePolicy) -> Result<()> {
        live_graph.node_mut(*self)?.resize_policy = value;
        Ok(())
    }
}

impl Translator<ResizeFilter> for NodeId {
    fn get(&self, live_graph: &LiveGraph) -> Result<ResizeFilter> {
        Ok(live_graph.node(*self)?.resize_filter)
    }

    fn set(&self, live_graph: &mut LiveGraph, value: ResizeFilter) -> Result<()> {
        live_graph.node_mut(*self)?.resize_filter = value;
        Ok(())
    }
}

impl Translator<ChannelPixel> for NodeId {
    fn get(&self, live_graph: &LiveGraph) -> Result<ChannelPixel> {
        let node = live_graph.node(*self)?;

        if let NodeType::Value(val) = node.node_type {
            Ok(val)
        } else {
            bail!("wrong NodeType: {:?}", node.node_type)
        }
    }

    fn set(&self, live_graph: &mut LiveGraph, value: ChannelPixel) -> Result<()> {
        let mut node = live_graph.node_mut(*self)?;

        if node.node_type == NodeType::Value(0.0) {
            node.node_type = NodeType::Value(value);
        } else {
            bail!("wrong NodeType: {:?}", node.node_type)
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct GuiUndoCommand<T, U>
where
    T: Debug + Clone,
    U: Translator<T>,
{
    contact_info: U,
    from: T,
    to: T,
}

impl<T: Debug + Clone, U: Translator<T>> UndoCommand for GuiUndoCommand<T, U> {
    fn forward(&self, world: &mut bevy::prelude::World, _: &mut UndoCommandManager) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                let _ = self.contact_info.set(&mut live_graph, self.to.clone());
            }
        }
    }

    fn backward(&self, world: &mut bevy::prelude::World, _: &mut UndoCommandManager) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                let _ = self.contact_info.set(&mut live_graph, self.from.clone());
            }
        }
    }
}

impl<T, U> GuiUndoCommand<T, U>
where
    T: Debug + Clone,
    U: Translator<T>,
{
    pub fn new(live_graph: &LiveGraph, contact_info: U, value: T) -> Self {
        let from = contact_info.get(live_graph).unwrap();

        Self {
            contact_info,
            from,
            to: value,
        }
    }
}
