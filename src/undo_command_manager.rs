use std::{collections::VecDeque, fmt::Debug};

use crate::Stage;
use bevy::prelude::*;

#[derive(Debug, Default)]
pub struct UndoCommandManager {
    commands: VecDeque<Box<dyn UndoCommand + Send + Sync + 'static>>,
    pub(crate) undo_stack: Vec<Box<dyn UndoCommand + Send + Sync + 'static>>,
    pub(crate) redo_stack: Vec<Box<dyn UndoCommand + Send + Sync + 'static>>,
}

impl UndoCommandManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, undo_command: Box<dyn UndoCommand + Send + Sync>) {
        self.commands.push_back(undo_command);
    }

    fn apply_commands(&mut self, world: &mut World) {
        while let Some(command) = self.commands.pop_front() {
            command.forward(world, self);

            if !command.custom() {
                self.undo_stack.push(command);
                self.redo_stack.clear();
            }
        }
    }

    pub fn undo_stack(&self) -> &Vec<Box<dyn UndoCommand + Send + Sync>> {
        &self.undo_stack
    }

    pub fn redo_stack(&self) -> &Vec<Box<dyn UndoCommand + Send + Sync>> {
        &self.redo_stack
    }
}

pub trait UndoCommand: Debug {
    fn custom(&self) -> bool {
        false
    }
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager);
    fn backward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager);
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
