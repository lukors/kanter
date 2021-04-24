use crate::box_select::*;
use crate::camera::*;
use crate::delete_tool::*;
use crate::deselect_tool::*;
use crate::drag_drop_entity::*;
use crate::hotkeys::*;
use crate::hoverable::*;
use crate::instructions::*;
use crate::material::*;
use crate::mouse_interaction::*;
use crate::processing::*;
use crate::scan_code_input::*;
use crate::sync_graph::*;
use crate::workspace::*;
use crate::{add_tool::*, ToolState};
use bevy::prelude::*;

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
