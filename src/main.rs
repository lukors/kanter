#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod add_tool;
pub mod box_select;
pub mod camera;
pub mod delete_tool;
pub mod deselect_tool;
pub mod drag_drop_entity;
pub mod hotkeys;
pub mod hoverable;
pub mod instructions;
pub mod kanter;
pub mod material;
pub mod mouse_interaction;
pub mod processing;
pub mod scan_code_input;
pub mod sync_graph;
pub mod workspace;

use bevy::{audio::AudioPlugin, prelude::*};
use camera::*;
use drag_drop_entity::*;
use hotkeys::*;
use hoverable::*;
use kanter::*;
use processing::*;
use sync_graph::*;
use workspace::*;

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
