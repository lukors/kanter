use crate::{AmbiguitySet, Stage};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    dag::TexProInt,
    node::{EmbeddedNodeDataId, Node, NodeType, ResizeFilter, ResizePolicy},
    node_graph::{NodeId, SlotId},
    slot_data::Size as TPSize,
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
    for node_id in tex_pro.get_all_clean() {
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
    let mut thumb_tex_pros_to_remove = Vec::new();

    for (node_id, tex_pro) in thumb_tex_pro.iter() {
        if let (Some((node_e, _)), Ok(tex_pro)) = (
            q_node.iter().find(|(_, nid)| *nid == node_id),
            tex_pro.inner().try_read(),
        ) {
            if let Some(texture) = get_output(&*tex_pro) {
                let texture_handle = textures.add(texture);

                if let Some((thumbnail_e, _)) = q_thumbnail
                    .iter()
                    .find(|(_, parent_e)| parent_e.0 == node_e)
                {
                    info!("Got thumbnail");
                    commands
                        .entity(thumbnail_e)
                        .insert(materials.add(texture_handle.into()));
                } else {
                    error!("Couldn't find a thumbnail entity for the GUI node");
                }

                thumb_tex_pros_to_remove.push(*node_id);
            }
        }
    }

    for node_id_to_remove in thumb_tex_pros_to_remove {
        if let Some(index) = thumb_tex_pro
            .iter()
            .position(|(node_id, _)| *node_id == node_id_to_remove)
        {
            thumb_tex_pro.remove(index);
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

    let node_datas = tex_pro.node_slot_data(node_id);

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
            .embed_slot_data_with_id(Arc::clone(node_data), EmbeddedNodeDataId(i as u32))
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
        if !thumb_tex_pro[i].1.processing() {
            dbg!("Does this run?");
            finished_thumbs.push(thumb_tex_pro.remove(i));
        }
    }

    finished_thumbs
}

// fn finished_thumbnails(thumb_tex_pro: &mut ResMut<Vec<ThumbTexPro>>) -> Vec<ThumbTexPro> {
//     thumb_tex_pro.iter().filter(|(_, tp)| !tp.processing()).cloned().collect::<Vec<ThumbTexPro>>()
// }

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

/// Gets the output of a given TextureProcessor.
fn get_output(tex_pro_int: &TexProInt) -> Option<Texture> {
    if let Some(output_id) = tex_pro_int.node_graph.external_output_ids().first() {
        if let (Ok(buffer), Some(size)) = (
            tex_pro_int.get_output(*output_id),
            tex_pro_int.get_node_data_size(*output_id),
        ) {
            Some(Texture::new(
                Extent3d::new(size.width as u32, size.height as u32, 1),
                TextureDimension::D2,
                buffer,
                TextureFormat::Rgba8Unorm,
            ))
        } else {
            trace!("Tried getting an output, but the output node didn't have any data");
            None
        }
    } else {
        error!("Tried getting an output, but the graph does not have any outputs");
        None
    }
}
