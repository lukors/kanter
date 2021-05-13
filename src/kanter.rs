use crate::ToolState;
use bevy::prelude::*;

pub(crate) struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_state(ToolState::None)
            .add_plugin(crate::scan_code_input::ScanCodeInputPlugin)
            .add_plugin(crate::add_tool::AddToolPlugin)
            .add_plugin(crate::drag_drop_entity::WorkspaceDragDropPlugin)
            .add_plugin(crate::processing::ProcessingPlugin)
            .add_plugin(crate::mouse_interaction::MouseInteractionPlugin)
            .add_plugin(crate::box_select::BoxSelectPlugin)
            .add_plugin(crate::camera::CameraPlugin)
            .add_plugin(crate::workspace::WorkspacePlugin)
            .add_plugin(crate::material::MaterialPlugin)
            .add_plugin(crate::sync_graph::SyncGraphPlugin)
            .add_plugin(crate::instruction::InstructionPlugin)
            .add_plugin(crate::deselect_tool::DeselectToolPlugin)
            .add_plugin(crate::delete_tool::DeleteToolPlugin)
            .add_plugin(crate::hotkeys::HotkeysPlugin)
            .add_plugin(crate::hoverable::HoverablePlugin)
            .add_plugin(crate::edit_node::EditNodePlugin)
            .add_plugin(crate::thumbnail::ThumbnailPlugin)
            .add_plugin(crate::export::ExportPlugin)
            // .add_plugin(crate::drag_drop_import::DragDropImport)
            ;
    }
}
