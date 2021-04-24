#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod add_tool;
pub mod box_select;
pub mod camera;
pub mod mouse_interaction;
pub mod processing;
pub mod scan_code_input;
pub mod drag_drop_entity;
pub mod workspace;
pub mod material;
pub mod sync_graph;
pub mod instructions;
pub mod deselect_tool;
pub mod delete_tool;
pub mod hotkeys;
pub mod hoverable;

use bevy::{audio::AudioPlugin, prelude::*};
use add_tool::*;
use box_select::*;
use camera::*;
use mouse_interaction::*;
use processing::*;
use scan_code_input::*;
use drag_drop_entity::*;
use workspace::*;
use material::*;
use sync_graph::*;
use instructions::*;
use deselect_tool::*;
use delete_tool::*;
use hotkeys::*;
use hoverable::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GrabToolType {
    Add,
    Node,
    Slot,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ToolState {
    None,
    Add,
    BoxSelect,
    Delete,
    Export,
    Grab(GrabToolType),
    Process,
}

impl Default for ToolState {
    fn default() -> Self {
        Self::None
    }
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Kanter".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
        // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
        .add_plugins_with(DefaultPlugins, |group| group.disable::<AudioPlugin>())
        .add_plugin(KanterPlugin)
        .run();
}

struct Crosshair;
struct Cursor;
struct Selected;
struct Draggable;
struct Dragged;
struct Dropped;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum Stage {
    Input,
    Update,
    Apply,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, AmbiguitySetLabel)]
struct AmbiguitySet;

pub struct KanterPlugin;

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
