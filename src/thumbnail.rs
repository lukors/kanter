use crate::{AmbiguitySet, Stage};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    node::{EmbeddedNodeDataId, Node, NodeType, ResizeFilter, ResizePolicy},
    node_data::Size as TPSize,
    node_graph::{NodeId, SlotId},
    texture_processor::TextureProcessor,
};
use std::sync::Arc;

type ThumbTexPro = (NodeId, TextureProcessor);

pub(crate) const THUMBNAIL_SIZE: f32 = 128.;
pub(crate) struct Thumbnail;

pub(crate) struct ThumbnailPlugin;

impl Plugin for ThumbnailPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_non_send_resource(Vec::<ThumbTexPro>::new())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(
                        generate_thumbnail_loop
                            .system()
                            .chain(get_thumbnail_loop.system())
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn generate_thumbnail_loop(
    q_node: Query<&NodeId>,
    tex_pro: Res<TextureProcessor>,
    mut thumb_tex_pro: ResMut<Vec<ThumbTexPro>>,
) {
    for node_id in tex_pro.clean_consume() {
        if q_node.iter().find(|nid| **nid == node_id).is_some() {
            generate_thumbnail(
                &tex_pro,
                &mut thumb_tex_pro,
                node_id,
                Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
            );
        }
    }
}

fn get_thumbnail_loop(
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    q_node: Query<(Entity, &NodeId)>,
    mut thumb_tex_pro: ResMut<Vec<ThumbTexPro>>,
) {
    for (node_id, tex_pro) in finished_thumbnails_consume(&mut thumb_tex_pro) {
        if let Some((node_e, _)) = q_node.iter().find(|(_, nid)| **nid == node_id) {
            if let Some(texture) = try_get_output(&tex_pro) {
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
    }
}

/// Creates a `TextureProcessor` which creates a thumbnail image from the data of a node
/// in a graph. It adds the `TextureProcessor` to the list of thumbnail processors
/// so the result can be retrieved and used in the future.
fn generate_thumbnail(
    tex_pro: &Res<TextureProcessor>,
    thumb_tex_pro: &mut ResMut<Vec<ThumbTexPro>>,
    node_id: NodeId,
    size: Size,
) {
    let tex_pro_thumb = TextureProcessor::new();

    let node_datas = tex_pro.get_node_data(node_id);

    let n_out = tex_pro_thumb
        .add_node(
            Node::new(NodeType::OutputRgba)
                .resize_policy(ResizePolicy::SpecificSize(TPSize::new(
                    size.width as u32,
                    size.height as u32,
                )))
                .resize_filter(ResizeFilter::Nearest),
        )
        .unwrap();

    for (i, node_data) in node_datas.iter().take(4).enumerate() {
        if let Ok(end_id) = tex_pro_thumb
            .embed_node_data_with_id(Arc::clone(node_data), EmbeddedNodeDataId(i as u32))
        {
            let n_node_data = tex_pro_thumb
                .add_node(Node::new(NodeType::NodeData(end_id)))
                .unwrap();

            tex_pro_thumb
                .connect(n_node_data, n_out, SlotId(0), node_data.slot_id)
                .unwrap()
        }
    }

    tex_pro_thumb.process();

    (*thumb_tex_pro).push((node_id, tex_pro_thumb));
}

fn finished_thumbnails_consume(thumb_tex_pro: &mut ResMut<Vec<ThumbTexPro>>) -> Vec<ThumbTexPro> {
    let mut finished_thumbs = Vec::new();

    for i in (0..thumb_tex_pro.len()).rev() {
        if thumb_tex_pro[i].1.finished() {
            finished_thumbs.push(thumb_tex_pro.remove(i));
        }
    }

    finished_thumbs
}

/// Tries to get the output of a given graph.
fn try_get_output(tex_pro: &TextureProcessor) -> Option<Texture> {
    if let Some(output_id) = tex_pro.external_output_ids().first() {
        if let (Ok(buffer), Some(size)) = (
            tex_pro.try_get_output(*output_id),
            tex_pro.get_node_data_size(*output_id),
        ) {
            Some(Texture::new(
                Extent3d::new(size.width as u32, size.height as u32, 1),
                TextureDimension::D2,
                buffer,
                TextureFormat::Rgba8Unorm,
            ))
        } else {
            None
        }
    } else {
        error!("Tried getting an output, but the graph does not have any outputs");
        None
    }
}
