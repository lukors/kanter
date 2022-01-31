#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod add_tool;
pub mod box_select;
pub mod camera;
pub mod core_translation;
pub mod delete_tool;
pub mod deselect_tool;
pub mod drag_drop;
// This will be re-enabled when drag and drop is supported in Bevy on Windows.
// pub mod drag_drop_import;
pub mod edit_node;
pub mod export;
pub mod export_outputs;
pub mod hotkeys;
pub mod hoverable;
pub mod instruction;
pub mod kanter;
pub mod listable;
pub mod material;
pub mod mouse_interaction;
pub mod node_state;
pub mod none_tool;
pub mod scan_code_input;
pub mod shared;
pub mod sync_graph;
pub mod thumbnail;
pub mod thumbnail_state;
pub mod undo;
pub mod workspace;

use bevy::prelude::*;
use camera::*;
use drag_drop::*;
use hotkeys::*;
use hoverable::*;
use kanter::*;
use mouse_interaction::select::Selected;
use sync_graph::*;
use workspace::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, AmbiguitySetLabel)]
pub(crate) struct AmbiguitySet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub(crate) enum CustomStage {
    /// Gathering input.
    Input,
    Setup,
    Update,
    /// Applying all changes.
    Apply,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum GrabToolType {
    Add,
    Node,
    Slot,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum ToolState {
    Add,
    BoxSelect,
    EditNode,
    Export,
    ExportOutputs,
    Grab(GrabToolType),
    None,
    Process,
    Redo,
    Undo,
}

impl Default for ToolState {
    fn default() -> Self {
        Self::None
    }
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Kanter".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: true,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
        // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
        .add_plugins(DefaultPlugins)
        .add_plugin(KanterPlugin)
        .run();
}
