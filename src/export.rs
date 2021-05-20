use crate::{
    instruction::ToolList, scan_code_input::ScanCodeInput, AmbiguitySet, Selected, Stage, ToolState,
};
use bevy::prelude::*;
use kanter_core::{
    node_graph::{NodeId, SlotId},
    slot_data::Size as TPSize,
    texture_processor::TextureProcessor,
};
use native_dialog::FileDialog;
use std::path::Path;

pub(crate) struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(
                        export
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Export))
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("Alt Shift S: Export selected".to_string());
}

fn export(
    tex_pro: Res<TextureProcessor>,
    q_selected: Query<&NodeId, With<Selected>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut keyboard_input: ResMut<ScanCodeInput>,
) {
    for node_id in q_selected.iter() {
        info!("1");
        let size: TPSize = match tex_pro.get_slot_data_size(*node_id, SlotId(0)) {
            Ok(s) => s,
            Err(_) => {
                info!("Unable to get the size of the node");
                continue;
            }
        };

        info!("2");
        let path = match FileDialog::new()
            // .set_location("~/Desktop")
            .add_filter("PNG Image", &["png"])
            .show_save_single_file()
        {
            Ok(path) => path,
            Err(e) => {
                warn!("Unable to get export path: {:?}\n", e);
                continue;
            }
        };

        info!("3");
        let path = match path {
            Some(path) => path,
            None => {
                warn!("Invalid export path");
                continue;
            }
        };

        info!("4");
        let texels = match tex_pro.try_get_output(*node_id) {
            Ok(buf) => buf,
            Err(e) => {
                error!("Error when trying to get pixels from image: {:?}", e);
                continue;
            }
        };

        info!("5");
        let buffer = match image::RgbaImage::from_vec(size.width, size.height, texels) {
            None => {
                error!("Output image buffer not big enough to contain texels.");
                continue;
            }
            Some(buf) => buf,
        };

        info!("6");
        match image::save_buffer(
            &Path::new(&path),
            &buffer,
            size.width,
            size.height,
            image::ColorType::RGBA(8),
        ) {
            Ok(_) => info!("Image exported to {:?}", path),
            Err(e) => {
                error!("{}", e);
                continue;
            }
        }
        info!("7");
    }

    keyboard_input.clear();
    tool_state.overwrite_replace(ToolState::None).unwrap();
}
