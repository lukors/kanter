use bevy::prelude::*;
use kanter_core::{dag::TextureProcessor, node_graph::NodeId};

use crate::{AmbiguitySet, Selected, Stage, ToolState};

pub(crate) struct DeleteToolPlugin;

impl Plugin for DeleteToolPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(
            CoreStage::Update,
            delete
                .system()
                .label(Stage::Update)
                .after(Stage::Input)
                .with_run_criteria(State::on_update(ToolState::Delete))
                .in_ambiguity_set(AmbiguitySet),
        );
    }
}

fn delete(
    mut tool_state: ResMut<State<ToolState>>,
    mut tex_pro: ResMut<TextureProcessor>,
    q_selected_nodes: Query<&NodeId, With<Selected>>,
) {
    for node_id in q_selected_nodes.iter() {
        match tex_pro.node_graph.remove_node(*node_id) {
            Ok(_) => (),
            Err(e) => warn!("Unable to remove node with id {}: {}", node_id, e),
        }
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
}
