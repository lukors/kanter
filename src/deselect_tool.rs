use bevy::prelude::*;

use crate::{
    instruction::ToolList,
    scan_code_input::{ScanCode, ScanCodeInput},
    AmbiguitySet, CustomStage, Selected, ToolState,
};

pub(crate) struct DeselectToolPlugin;

impl Plugin for DeselectToolPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_to_stage(
                CoreStage::Update,
                deselect
                    .system()
                    .label(CustomStage::Apply)
                    .after(CustomStage::Update)
                    .with_run_criteria(State::on_update(ToolState::None)),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("A: Deselect all".to_string());
}

fn deselect(
    input: Res<ScanCodeInput>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if input.just_pressed(ScanCode::KeyA) {
        for entity in q_selected.iter() {
            commands.entity(entity).remove::<Selected>();
        }
    }
}
