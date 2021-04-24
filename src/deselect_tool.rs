use bevy::prelude::*;

use crate::{
    scan_code_input::{ScanCode, ScanCodeInput},
    Selected, Stage,
};

pub(crate) struct DeselectToolPlugin;

impl Plugin for DeselectToolPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(
            CoreStage::Update,
            deselect.system().label(Stage::Apply).after(Stage::Update),
        );
    }
}

/// This function should be turned into a tool and the hotkey should live in the hotkey system.
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
