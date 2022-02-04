use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use vismut_core::{live_graph::LiveGraph, node::node_type::NodeType};

use crate::{
    add_tool::create_and_grab_node,
    camera::Cursor,
    instruction::ToolList,
    mouse_interaction::Selected,
    sync_graph::NODE_SIZE,
    undo::{prelude::UndoCommandManager, UndoCommand},
    AmbiguitySet, CustomStage, ToolState,
};

pub(crate) struct DragDropImport;

impl Plugin for DragDropImport {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new().label(CustomStage::Input).with_system(
                    drag_drop_import
                        .system()
                        .with_run_criteria(State::on_update(ToolState::None))
                        .in_ambiguity_set(AmbiguitySet),
                ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("Drag and drop to import image".to_string());
}

fn drag_drop_import(
    mut undo_command_manager: ResMut<UndoCommandManager>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut events: EventReader<FileDragAndDrop>,
) {
    let mut created_nodes: usize = 0;

    for event in events.iter() {
        if let FileDragAndDrop::DroppedFile { id: _, path_buf } = event {
            let node_type = NodeType::Image(path_buf.clone());

            if create_and_grab_node(&mut undo_command_manager, &*live_graph, &node_type).is_ok() {
                created_nodes += 1;
            } else {
                error!("failed to create node: {:?}", node_type);
            }
        }
    }

    if created_nodes > 1 {
        undo_command_manager.push(Box::new(MultiImportOffset));
    }
}
