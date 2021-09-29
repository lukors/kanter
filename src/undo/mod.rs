pub mod edge;
pub mod gui;
pub mod node;
pub mod prelude;
pub mod undo_batch;
pub mod undo_command_manager;
pub mod undo_redo_tool;

use self::undo_command_manager::UndoCommandManager;
use bevy::prelude::World;
use std::fmt::Debug;

trait AddRemove: Debug {
    fn add(&self, world: &mut World);
    fn remove(&self, world: &mut World);
}

pub trait UndoCommand: Debug {
    fn custom(&self) -> bool {
        false
    }
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager);
    fn backward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager);
}
