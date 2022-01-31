use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use kanter_core::{
    error::TexProError, live_graph::LiveGraph, node::node_type::NodeType, node_graph::SlotId,
    slot_data::Size as CoreSize,
};
use native_dialog::FileDialog;

use crate::{
    instruction::ToolList,
    scan_code_input::{ScanCode, ScanCodeInput},
    AmbiguitySet, ToolState,
};

pub(crate) struct ExportOutputsToolPlugin;

impl Plugin for ExportOutputsToolPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new().with_system(
                    export_outputs
                        .with_run_criteria(State::on_enter(ToolState::ExportOutputs))
                        .in_ambiguity_set(AmbiguitySet),
                ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("Ctrl (Shift) E: Export outputs".to_string());
}

fn export_outputs(
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut sc_input: ResMut<ScanCodeInput>,
) {
    let directory = match FileDialog::new().show_open_single_dir() {
        Ok(path) => path,
        Err(e) => {
            warn!("Unable to get export directory: {:?}\n", e);
            None
        }
    };

    if let Some(path) = directory {
        let output_ids = live_graph.read().unwrap().output_ids();

        for node_id in output_ids {
            let live_graph = LiveGraph::await_clean_read(&live_graph, node_id).unwrap();
            let mut path = path.clone();

            let size: CoreSize = match live_graph.slot_data_size(node_id, SlotId(0)) {
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

            let file_name = if let NodeType::OutputRgba(file_name) =
                live_graph.node(node_id).unwrap().node_type
            {
                file_name
            } else {
                error!("could not get name of output node with ID: {}", node_id);
                continue;
            };

            path.push(file_name);
            path.set_extension("png");

            let texels = match live_graph.buffer_rgba(node_id, SlotId(0)) {
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
                &path,
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
    } else {
        info!("cancelled file dialog");
    }

    sc_input.reset_vec(vec![
        ScanCode::ControlLeft,
        ScanCode::ControlRight,
        ScanCode::ShiftLeft,
        ScanCode::ShiftRight,
        ScanCode::KeyE,
    ]);

    tool_state.overwrite_replace(ToolState::None).unwrap();
}
