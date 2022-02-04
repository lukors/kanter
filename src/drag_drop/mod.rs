pub mod edge;
pub mod node;

use crate::{AmbiguitySet, GrabToolType, ToolState, instruction::ToolList};
use bevy::prelude::*;

use self::{
    edge::grab_edge_cleanup,
    node::{grab_node_setup, grab_node_update_edge},
};
use self::{
    edge::{grab_edge_update, grab_tool_slot_setup},
    node::{grab_node_cleanup, grab_node_update},
};

#[derive(Component, Default)]
pub(crate) struct Draggable;
#[derive(Component, Default)]
pub(crate) struct Dragged {
    start: Vec2,
}
#[derive(Component, Default)]
pub(crate) struct Dropped;

pub(crate) struct WorkspaceDragDropPlugin;

impl Plugin for WorkspaceDragDropPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .with_system(
                    grab_node_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_node_update
                        .system()
                        .chain(grab_node_update_edge)
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_node_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        )
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .with_system(
                    grab_tool_slot_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_edge_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_edge_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("G: Move".to_string());
}