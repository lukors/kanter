use crate::{scan_code_input::ScanCodeInput, AmbiguitySet, Selected, Stage, ToolState};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    dag::TextureProcessor,
    node::{EmbeddedNodeDataId, Node, NodeType, ResizeFilter, ResizePolicy},
    node_data::Size as TPSize,
    node_graph::{NodeId, SlotId},
};
use native_dialog::FileDialog;
/// Texture Processing
use std::{path::Path, sync::Arc};

pub(crate) const THUMBNAIL_SIZE: f32 = 128.;
pub(crate) struct Thumbnail;

pub(crate) struct ProcessingPlugin;

impl Plugin for ProcessingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Apply)
                .after(Stage::Update)
                .with_system(
                    process
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Process))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    export
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Export))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        );
    }
}

fn process(
    mut tex_pro: ResMut<TextureProcessor>,
    mut tool_state: ResMut<State<ToolState>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    q_node: Query<(Entity, &NodeId)>,
) {
    info!("Processing graph...");
    tex_pro.process();

    info!("Generating thumbnails...");
    for (node_e, node_id) in q_node.iter() {
        if let Some(texture) = generate_thumbnail(
            &tex_pro,
            *node_id,
            Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
        ) {
            let texture_handle = textures.add(texture);

            if let Some((thumbnail_e, _)) = q_thumbnail
                .iter()
                .find(|(_, parent_e)| parent_e.0 == node_e)
            {
                commands
                    .entity(thumbnail_e)
                    .insert(materials.add(texture_handle.into()));
            }
        }
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
    info!("Done");
}

fn generate_thumbnail(
    tex_pro: &ResMut<TextureProcessor>,
    node_id: NodeId,
    size: Size,
) -> Option<Texture> {
    let mut tex_pro_thumb = TextureProcessor::new();

    let node_datas = tex_pro.get_node_data(node_id);

    let n_out = tex_pro_thumb
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    for (i, node_data) in node_datas.iter().take(4).enumerate() {
        if let Ok(end_id) = tex_pro_thumb
            .embed_node_data_with_id(Arc::clone(node_data), EmbeddedNodeDataId(i as u32))
        {
            let n_node_data = tex_pro_thumb
                .node_graph
                .add_node(Node::new(NodeType::NodeData(end_id)))
                .unwrap();

            let n_resize = tex_pro_thumb
                .node_graph
                .add_node(Node::new(NodeType::Resize(
                    Some(ResizePolicy::SpecificSize(TPSize::new(
                        size.width as u32,
                        size.height as u32,
                    ))),
                    Some(ResizeFilter::Nearest),
                )))
                .unwrap();

            tex_pro_thumb
                .node_graph
                .connect(n_node_data, n_resize, SlotId(0), SlotId(0))
                .unwrap();

            tex_pro_thumb
                .node_graph
                .connect(n_resize, n_out, SlotId(0), node_data.slot_id)
                .unwrap()
        }
    }

    tex_pro_thumb.process();

    if let Ok(output) = tex_pro_thumb.get_output(n_out) {
        Some(Texture::new(
            Extent3d::new(size.width as u32, size.height as u32, 1),
            TextureDimension::D2,
            output,
            TextureFormat::Rgba8Unorm,
        ))
    } else {
        None
    }
}

fn export(
    tex_pro: Res<TextureProcessor>,
    q_selected: Query<&NodeId, With<Selected>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut keyboard_input: ResMut<ScanCodeInput>,
) {
    for node_id in q_selected.iter() {
        let size: TPSize = match tex_pro.get_node_size(*node_id) {
            Some(s) => s,
            None => {
                info!("Unable to get the size of the node");
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

        let texels = match tex_pro.get_output(*node_id) {
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

    keyboard_input.clear();
    tool_state.overwrite_replace(ToolState::None).unwrap();
}
