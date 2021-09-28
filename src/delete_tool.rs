use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use kanter_core::{live_graph::LiveGraph, node::Node, node_graph::NodeId};

use crate::{
    instruction::ToolList,
    undo::undo_command_manager::{UndoCommand, UndoCommandManager},
    AmbiguitySet, Selected, Stage, ToolState,
};

pub(crate) struct DeleteToolPlugin;

impl Plugin for DeleteToolPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_to_stage(
                CoreStage::Update,
                delete
                    .system()
                    .label(Stage::Setup)
                    .after(Stage::Input)
                    .with_run_criteria(State::on_update(ToolState::Delete))
                    .in_ambiguity_set(AmbiguitySet),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("X: Delete node".to_string());
}

fn delete(
    mut tool_state: ResMut<State<ToolState>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    q_selected_nodes: Query<&NodeId, With<Selected>>,
) {
    for node_id in q_selected_nodes.iter() {
        match live_graph.write().unwrap().remove_node(*node_id) {
            Ok(_) => (),
            Err(e) => warn!("Unable to remove node with id {}: {}", node_id, e),
        }
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
}
