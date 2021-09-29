use std::fmt::Debug;

use super::prelude::*;
use crate::{AmbiguitySet, Stage, ToolState};
use bevy::prelude::*;

#[derive(Debug)]
pub struct Undo;
impl UndoCommand for Undo {
    fn custom(&self) -> bool {
        true
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        if let Some(command) = undo_command_manager.undo_stack.pop() {
            command.backward(world, undo_command_manager);
            undo_command_manager.redo_stack.push(command);
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!()
    }
}

#[derive(Debug)]
pub struct Redo;
impl UndoCommand for Redo {
    fn custom(&self) -> bool {
        true
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        if let Some(command) = undo_command_manager.redo_stack.pop() {
            command.forward(world, undo_command_manager);
            undo_command_manager.undo_stack.push(command);
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!()
    }
}

pub struct UndoPlugin;
impl Plugin for UndoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(
            CoreStage::Update,
            undo.system()
                .label(Stage::Update)
                .after(Stage::Setup)
                .with_run_criteria(State::on_update(ToolState::Undo))
                .in_ambiguity_set(AmbiguitySet),
        )
        .add_system_to_stage(
            CoreStage::Update,
            redo.system()
                .label(Stage::Update)
                .after(Stage::Setup)
                .with_run_criteria(State::on_update(ToolState::Redo))
                .in_ambiguity_set(AmbiguitySet),
        );
    }
}

fn undo(
    mut tool_state: ResMut<State<ToolState>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    undo_command_manager.push(Box::new(Undo));
    tool_state.overwrite_replace(ToolState::None).unwrap();
}

fn redo(
    mut tool_state: ResMut<State<ToolState>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    undo_command_manager.push(Box::new(Redo));
    tool_state.overwrite_replace(ToolState::None).unwrap();
}
