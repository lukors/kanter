use bevy::prelude::*;
use crate::{ToolState, add_tool::*};
use crate::box_select::*;
use crate::camera::*;
use crate::mouse_interaction::*;
use crate::processing::*;
use crate::scan_code_input::*;
use crate::drag_drop_entity::*;
use crate::workspace::*;
use crate::material::*;
use crate::sync_graph::*;
use crate::instructions::*;
use crate::deselect_tool::*;
use crate::delete_tool::*;
use crate::hotkeys::*;
use crate::hoverable::*;

pub(crate) struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_state(ToolState::None)
            .add_state(FirstPersonState::Off)
            .add_plugin(ScanCodeInputPlugin)
            .add_plugin(AddToolPlugin)
            .add_plugin(WorkspaceDragDropPlugin)
            .add_plugin(ProcessingPlugin)
            .add_plugin(MouseInteractionPlugin)
            .add_plugin(BoxSelectPlugin)
            .add_plugin(CameraPlugin)
            .add_plugin(WorkspacePlugin)
            .add_plugin(MaterialPlugin)
            .add_plugin(SyncGraphPlugin)
            .add_plugin(InstructionPlugin)
            .add_plugin(DeselectToolPlugin)
            .add_plugin(DeleteToolPlugin)
            .add_plugin(HotkeysPlugin)
            .add_plugin(HoverablePlugin);
    }
}