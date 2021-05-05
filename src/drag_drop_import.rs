use bevy::prelude::*;

use crate::{instruction::ToolList, AmbiguitySet, Stage, ToolState};

pub(crate) struct DragDropImport;

impl Plugin for DragDropImport {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new().label(Stage::Input).with_system(
                    drag_drop_import
                        .system()
                        .with_run_criteria(State::on_update(ToolState::None))
                        .in_ambiguity_set(AmbiguitySet),
                ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("Drag and drop to import image".to_string());
}

fn drag_drop_import(mut events: EventReader<FileDragAndDrop>) {
    for event in events.iter() {
        info!("{:?}", event);
    }
}
