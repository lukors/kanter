pub mod edge;
pub mod gui;
pub mod node;
pub mod prelude;
pub mod slot;
pub mod undo_command_manager;
pub mod undo_redo_tool;

use self::undo_command_manager::UndoCommandManager;
use bevy::prelude::*;
use std::fmt::Debug;

trait AddRemove: Debug {
    fn add(&self, world: &mut World);
    fn remove(&self, world: &mut World);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UndoCommandType {
    Command,
    Custom,
    Checkpoint,
}

pub trait UndoCommand: Debug {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Command
    }
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager);
    fn backward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager);
}
