use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use anyhow::{anyhow, bail, Result};
use kanter_core::{
    live_graph::LiveGraph,
    node::{node_type::NodeType, ResizeFilter},
    node_graph::NodeId,
    slot_data::ChannelPixel,
};

use crate::undo_command_manager::{UndoCommand, UndoCommandManager};

pub trait Translator<DataType>: Debug {
    fn get(&self, live_graph: &Arc<RwLock<LiveGraph>>) -> Result<DataType>;
    fn set(&self, live_graph: &Arc<RwLock<LiveGraph>>, value: DataType) -> Result<()>;
}

impl Translator<ResizeFilter> for NodeId {
    fn get(&self, live_graph: &Arc<RwLock<LiveGraph>>) -> Result<ResizeFilter> {
        let live_graph = live_graph
            .read()
            .map_err(|_| anyhow!("unable to get read lock on LiveGraph"))?;
        Ok(live_graph.node(*self)?.resize_filter)
    }

    fn set(&self, live_graph: &Arc<RwLock<LiveGraph>>, value: ResizeFilter) -> Result<()> {
        let mut live_graph = live_graph
            .write()
            .map_err(|_| anyhow!("unable to get write lock on LiveGraph"))?;
        live_graph.node_mut(*self)?.resize_filter = value;
        Ok(())
    }
}

impl Translator<ChannelPixel> for NodeId {
    fn get(&self, live_graph: &Arc<RwLock<LiveGraph>>) -> Result<ChannelPixel> {
        let live_graph = live_graph
            .read()
            .map_err(|_| anyhow!("unable to get read lock on LiveGraph"))?;
        let node = live_graph.node(*self)?;

        if let NodeType::Value(val) = node.node_type {
            Ok(val)
        } else {
            bail!("wrong NodeType: {:?}", node.node_type)
        }
    }

    fn set(&self, live_graph: &Arc<RwLock<LiveGraph>>, value: ChannelPixel) -> Result<()> {
        let mut live_graph = live_graph
            .write()
            .map_err(|_| anyhow!("unable to get write lock on LiveGraph"))?;
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
pub struct GuiTranslator<T, U>
where
    T: Debug + Clone,
    U: Translator<T>,
{
    contact_info: U,
    from: T,
    to: T,
}

impl<T: Debug + Clone, U: Translator<T>> UndoCommand for GuiTranslator<T, U> {
    fn forward(&self, world: &mut bevy::prelude::World, _: &mut UndoCommandManager) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            let _ = self.contact_info.set(live_graph, self.to.clone());
        }
    }

    fn backward(&self, world: &mut bevy::prelude::World, _: &mut UndoCommandManager) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            let _ = self.contact_info.set(live_graph, self.from.clone());
        }
    }
}

impl<T, U> GuiTranslator<T, U>
where
    T: Debug + Clone,
    U: Translator<T>,
{
    pub fn new(contact_info: U, from: T, to: T) -> Self {
        Self {
            contact_info,
            from,
            to,
        }
    }
}
