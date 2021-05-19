use crate::{AmbiguitySet, Stage};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    node::{EmbeddedNodeDataId, Node, NodeType, ResizeFilter, ResizePolicy},
    node_graph::{NodeId, SlotId},
    slot_data::Size as TPSize,
    texture_processor::TextureProcessor,
};
use std::sync::Arc;

type TexProThumb = (NodeId, TextureProcessor);

pub(crate) const THUMBNAIL_SIZE: f32 = 128.;
pub(crate) struct Thumbnail;

pub(crate) struct ThumbnailPlugin;

#[derive(Debug, PartialEq)]
pub(crate) enum ThumbnailState {
    Missing,
    Processing,
    Present,
}

impl Plugin for ThumbnailPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_non_send_resource(Vec::<TexProThumb>::new())
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
    mut q_node: Query<(&NodeId, &mut ThumbnailState)>,
    tex_pro: Res<TextureProcessor>,
    mut thumb_tex_pro: ResMut<Vec<TexProThumb>>,
) {
    for (node_id, mut thumb_state) in q_node
        .iter_mut()
        .filter(|(_, state)| **state == ThumbnailState::Missing)
    {
        generate_thumbnail(
            &tex_pro,
            &mut thumb_tex_pro,
            *node_id,
            Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
        );
        *thumb_state = ThumbnailState::Processing;
    }
}

fn get_thumbnail_loop(
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    mut q_node: Query<(Entity, &NodeId, &mut ThumbnailState)>,
    mut thumb_tex_pro: ResMut<Vec<TexProThumb>>,
) {
    let mut thumb_tex_pros_to_remove = Vec::new();

    for (index, (node_id, tex_pro)) in thumb_tex_pro.iter().enumerate() {
        if let Some((node_e, _, mut thumb_state)) =
            q_node.iter_mut().find(|(_, nid, _)| *nid == node_id)
        {
            if let Some(texture) = try_get_output(&*tex_pro) {
                let texture_handle = textures.add(texture);

                if let Some((thumbnail_e, _)) = q_thumbnail
                    .iter()
                    .find(|(_, parent_e)| parent_e.0 == node_e)
                {
                    trace!("Got thumbnail");
                    commands
                        .entity(thumbnail_e)
                        .insert(materials.add(texture_handle.into()));
                } else {
                    error!("Couldn't find a thumbnail entity for the GUI node");
                }

                thumb_tex_pros_to_remove.push(index);
                *thumb_state = ThumbnailState::Present;
            }
        }
    }

    for index in thumb_tex_pros_to_remove.into_iter().rev() {
        thumb_tex_pro.remove(index);
    }
}

/// Creates a `TextureProcessor` which creates a thumbnail image from the data of a node
/// in a graph. It adds the `TextureProcessor` to the list of thumbnail processors
/// so the result can be retrieved and used in the future.
///
/// If a processor already exists for a node, throw away the old one.
fn generate_thumbnail(
    tex_pro: &Res<TextureProcessor>,
    tex_pro_thumbs: &mut ResMut<Vec<TexProThumb>>,
    node_id: NodeId,
    size: Size,
) {
    // Remove any existing thumbnail processor for the node.
    if let Some(index) = tex_pro_thumbs
        .iter()
        .map(|(node_id_tp, _)| *node_id_tp)
        .position(|node_id_tp| node_id_tp == node_id)
    {
        tex_pro_thumbs.remove(index);
    }

    let tex_pro_thumb = TextureProcessor::new();

    // Todo: If there's stutter when thumbnails are generated it's probably from here.
    let node_datas = tex_pro.node_slot_data(node_id).unwrap();

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

    tex_pro_thumb.process_then_kill();

    (*tex_pro_thumbs).push((node_id, tex_pro_thumb));
}

/// Tries to get the output of a given graph.
fn try_get_output(tex_pro: &TextureProcessor) -> Option<Texture> {
    let output_id = *tex_pro.external_output_ids().first()?;
    let buffer = tex_pro.try_get_output(output_id).ok()?;
    let size = tex_pro.try_get_slot_data_size(output_id, SlotId(0)).ok()?;

    Some(Texture::new(
        Extent3d::new(size.width as u32, size.height as u32, 1),
        TextureDimension::D2,
        buffer,
        TextureFormat::Rgba8Unorm,
    ))
}
