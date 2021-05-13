use crate::{instruction::ToolList, AmbiguitySet, Stage, ToolState};
use bevy::prelude::*;
use kanter_core::texture_processor::TextureProcessor;

pub(crate) struct ProcessingPlugin;

impl Plugin for ProcessingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(
                        process
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Process))
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("F12: Process graph".to_string());
}

fn process(tex_pro: ResMut<TextureProcessor>, mut tool_state: ResMut<State<ToolState>>) {
    info!("Starting graph processing...");
    tex_pro.process();
    tool_state.overwrite_replace(ToolState::None).unwrap();
}
