use super::UndoCommand;

impl UndoCommand for Vec<Box<dyn UndoCommand>> {
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
