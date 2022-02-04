use crate::{
    instruction::ToolList, shared::NodeIdComponent, AmbiguitySet, CustomStage, Selected, ToolState,
};
use bevy::prelude::*;
use native_dialog::FileDialog;
use std::{
    path::Path,
    sync::{Arc, RwLock},
};
use vismut_core::{
    error::TexProError, live_graph::LiveGraph, node_graph::SlotId, slot_data::Size as TPSize,
};

pub(crate) struct ExportPlugin;

impl Plugin for ExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(CustomStage::Apply)
                    .after(CustomStage::Update)
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
    tool_list.insert("Shift Alt E: Export active".to_string());
}

fn export(
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    q_selected: Query<&NodeIdComponent, With<Selected>>,
    mut tool_state: ResMut<State<ToolState>>,
) {
    for node_id in q_selected.iter() {
        let _result = LiveGraph::await_clean_read(&live_graph, node_id.0);

        let size: TPSize = match LiveGraph::await_clean_read(&live_graph, node_id.0)
            .unwrap()
            .slot_data_size(node_id.0, SlotId(0))
        {
            Ok(s) => s,
            Err(TexProError::InvalidBufferCount) => {
                warn!("Seems the node doesn't have any outputs");
                continue;
            }
            Err(e) => {
                error!("Unable to get the size of the node: {}", e);
                continue;
            }
        };

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

        let path = match path {
            Some(path) => path,
            None => {
                warn!("Invalid export path");
                continue;
            }
        };

        let texels = match live_graph.read().unwrap().buffer_rgba(node_id.0, SlotId(0)) {
            Ok(buf) => buf,
            Err(e) => {
                error!("Error when trying to get pixels from image: {:?}", e);
                continue;
            }
        };

        let buffer = match image::RgbaImage::from_vec(size.width, size.height, texels) {
            None => {
                error!("Output image buffer not big enough to contain texels.");
                continue;
            }
            Some(buf) => buf,
        };

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
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
}
