use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use kanter_core::live_graph::LiveGraph;

use crate::{
    instruction::ToolList,
    undo::{node::RemoveNode, prelude::*},
    AmbiguitySet, Selected, Stage, ToolState, shared::NodeIdComponent,
};

pub(crate) struct DeleteToolPlugin;

impl Plugin for DeleteToolPlugin {
    fn build(&self, app: &mut App) {
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
    q_selected_nodes: Query<&NodeIdComponent, With<Selected>>,
) {
    for node_id in q_selected_nodes.iter() {
        match live_graph.write().unwrap().remove_node(node_id.0) {
            Ok(_) => (),
            Err(e) => warn!("Unable to remove node with id {}: {}", node_id.0, e),
        }
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
}

#[derive(Copy, Clone, Debug)]
pub struct DeleteSelected;
impl UndoCommand for DeleteSelected {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(
        &self,
        world: &mut World,
        undo_command_manager: &mut crate::undo::prelude::UndoCommandManager,
    ) {
        let mut query =
            world.query_filtered::<(&NodeIdComponent, &Transform), (With<Selected>, With<NodeIdComponent>)>();
        let live_graph = world
            .get_resource::<Arc<RwLock<LiveGraph>>>()
            .unwrap()
            .read()
            .unwrap();

        for (node_id, transform) in query.iter(world) {
            let node = live_graph.node(node_id.0).unwrap();
            let translation = transform.translation.truncate();

            undo_command_manager
                .commands
                .push_front(Box::new(RemoveNode::new(node.clone(), translation)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut crate::undo::prelude::UndoCommandManager) {
        unreachable!("this command is never stored in the undo stack");
    }
}
