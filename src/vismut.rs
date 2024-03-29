use std::sync::Arc;

use crate::ToolState;
use bevy::prelude::*;
use vismut_core::texture_processor::TextureProcessor;

pub(crate) struct VismutPlugin;

impl Plugin for VismutPlugin {
    fn build(&self, app: &mut App) {
        let tex_pro = TextureProcessor::new(Arc::new(1_000_000_000.into()));

        app.insert_non_send_resource(tex_pro)
            .add_state(ToolState::None)
            .add_plugin(crate::export_outputs::ExportOutputsToolPlugin)
            .add_plugin(crate::scan_code_input::ScanCodeInputPlugin)
            .add_plugin(crate::add_tool::AddToolPlugin)
            .add_plugin(crate::drag_drop::WorkspaceDragDropPlugin)
            .add_plugin(crate::mouse_interaction::MouseInteractionPlugin)
            .add_plugin(crate::box_select::BoxSelectPlugin)
            .add_plugin(crate::camera::CameraPlugin)
            .add_plugin(crate::workspace::WorkspacePlugin)
            .add_plugin(crate::material::MaterialPlugin)
            .add_plugin(crate::sync_graph::SyncGraphPlugin)
            .add_plugin(crate::instruction::InstructionPlugin)
            // .add_plugin(crate::deselect_tool::DeselectToolPlugin)
            .add_plugin(crate::delete_tool::DeleteToolPlugin)
            .add_plugin(crate::hotkeys::HotkeysPlugin)
            .add_plugin(crate::hoverable::HoverablePlugin)
            .add_plugin(crate::edit_node::EditNodePlugin)
            .add_plugin(crate::thumbnail::ThumbnailPlugin)
            .add_plugin(crate::export::ExportPlugin)
            .add_plugin(crate::none_tool::NoneToolPlugin)
            .add_plugin(crate::node_state::NodeStatePlugin)
            .add_plugin(crate::thumbnail_state::ThumbnailStatePlugin)
            .add_plugin(crate::undo::undo_command_manager::UndoCommandManagerPlugin)
            .add_plugin(crate::undo::undo_redo_tool::UndoPlugin);
        // This will be re-enabled when drag and drop is supported in Bevy on Windows.
        // .add_plugin(crate::drag_drop_import::DragDropImport);
    }
}
