use std::fmt::Debug;

use anyhow::{bail, Result};
use kanter_core::{
    live_graph::LiveGraph,
    node::{node_type::NodeType, ResizeFilter, ResizePolicy},
    node_graph::NodeId,
    slot_data::ChannelPixel,
};

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
