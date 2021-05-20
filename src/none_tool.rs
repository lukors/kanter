use crate::{AmbiguitySet, Stage, ToolState, instruction::*, scan_code_input::ScanCodeInput};
use bevy::prelude::*;

pub(crate) struct NoneToolPlugin;

impl Plugin for NoneToolPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Update)
                .after(Stage::Input)
                .with_run_criteria(State::on_enter(ToolState::None))
                .in_ambiguity_set(AmbiguitySet)
                .with_system(restore_instructions.system())
                .with_system(reset_buttonpresses.system())
        );
    }
}

fn restore_instructions(mut instructions: ResMut<Instructions>) {
    instructions.clear();
}

fn reset_buttonpresses(mut keyboard_input: ResMut<ScanCodeInput>) {
    keyboard_input.clear();
}
