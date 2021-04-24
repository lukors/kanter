#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod kanter;
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
use kanter::*;
use camera::*;
use processing::*;
use drag_drop_entity::*;
use workspace::*;
use sync_graph::*;
use hotkeys::*;
use hoverable::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, AmbiguitySetLabel)]
pub(crate) struct AmbiguitySet;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub(crate) enum Stage {
    Input,
    Update,
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
pub(crate) struct Selected;

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


