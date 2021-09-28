pub mod edge;
pub mod gui;
pub mod node;
pub mod undo_command_manager;
pub mod undo_redo_tool;

use bevy::prelude::World;
use std::fmt::Debug;

trait AddRemove: Debug {
    fn add(&self, world: &mut World);
    fn remove(&self, world: &mut World);
}
