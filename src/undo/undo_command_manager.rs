use crate::Stage;
use anyhow::{bail, Result};
use bevy::prelude::*;
use std::{collections::VecDeque, fmt::Debug};

use super::{UndoCommand, UndoCommandType};

type BoxUndoCommand = Box<dyn UndoCommand + Send + Sync + 'static>;

#[derive(Debug, Default)]
pub struct UndoCommandManager {
    pub(crate) commands: VecDeque<BoxUndoCommand>,
    pub(crate) undo_stack: Vec<BoxUndoCommand>,
    pub(crate) redo_stack: Vec<BoxUndoCommand>,
    pub(crate) command_batch: VecDeque<BoxUndoCommand>, // Maybe this and also the undo/redo stacks should be made private with some refactoring?
}

impl UndoCommandManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, undo_command: BoxUndoCommand) {
        self.commands.push_back(undo_command);
    }

    fn apply_commands(&mut self, world: &mut World) {
        // if !self.commands.is_empty() {
        //     dbg!(&self.commands);
        //     dbg!(&self.undo_stack);
        //     dbg!(&self.redo_stack);
        // }

        while let Some(command) = self.commands.pop_front() {
            command.forward(world, self);

            if command.command_type() == UndoCommandType::Command {
                self.command_batch.push_back(command);
                self.redo_stack.clear();
            }
        }
    }

    pub fn undo_stack(&self) -> &Vec<BoxUndoCommand> {
        &self.undo_stack
    }

    pub fn redo_stack(&self) -> &Vec<BoxUndoCommand> {
        &self.redo_stack
    }
}

pub struct UndoCommandManagerPlugin;
impl Plugin for UndoCommandManagerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_non_send_resource(UndoCommandManager::new())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(apply_commands.exclusive_system()),
            );
    }
}

fn apply_commands(world: &mut World) {
    if let Some(mut undo_command_manager) = world.remove_resource::<UndoCommandManager>() {
        undo_command_manager.apply_commands(world);
        world.insert_resource(undo_command_manager);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint;
impl UndoCommand for Checkpoint {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Checkpoint
    }

    fn forward(&self, _: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut command_vec: Vec<BoxUndoCommand> = Vec::new();

        while let Some(command) = undo_command_manager.command_batch.pop_front() {
            if command.command_type() != UndoCommandType::Command {
                undo_command_manager.command_batch.push_front(command);
                break;
            }
            command_vec.push(command);
        }

        undo_command_manager.undo_stack.push(Box::new(command_vec));
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("a `Checkpoint` is never put on the `undo_stack`")
    }
}

impl UndoCommand for Vec<BoxUndoCommand> {
    fn forward(
        &self,
        world: &mut bevy::prelude::World,
        undo_command_manager: &mut super::undo_command_manager::UndoCommandManager,
    ) {
        for command in self.iter() {
            command.forward(world, undo_command_manager);
        }
    }

    fn backward(
        &self,
        world: &mut bevy::prelude::World,
        undo_command_manager: &mut super::undo_command_manager::UndoCommandManager,
    ) {
        for command in self.iter().rev() {
            command.backward(world, undo_command_manager);
        }
    }
}
