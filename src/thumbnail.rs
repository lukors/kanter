use crate::{AmbiguitySet, Stage};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    error::TexProError,
    node::{EmbeddedNodeDataId, Node, NodeType, ResizeFilter, ResizePolicy, Side},
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
    Waiting,
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
                        get_thumbnail_loop
                            .system()
                            .chain(generate_thumbnail_loop.system())
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn generate_thumbnail_loop(
    mut commands: Commands,
    mut q_node: Query<(Entity, &NodeId, &mut ThumbnailState)>,
    tex_pro: Res<TextureProcessor>,
) {
    for (entity, node_id, mut thumb_state) in q_node
        .iter_mut()
        .filter(|(_, _, state)| **state == ThumbnailState::Missing)
    {
        commands.entity(entity).insert(thumbnail_processor(
            &tex_pro,
            *node_id,
            Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
        ));
        *thumb_state = ThumbnailState::Processing;
    }
}

fn get_thumbnail_loop(
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    mut q_node: Query<(Entity, &NodeId, &mut ThumbnailState, &TextureProcessor)>,
) {
    for (node_e, node_id, mut thumb_state, tex_pro) in q_node.iter_mut() {
        let material = match try_get_output(&*tex_pro) {
            Ok(texture) => {
                let texture_handle = textures.add(texture);
                Some(materials.add(texture_handle.into()))
            }
            Err(TexProError::InvalidBufferCount) => {
                Some(materials.add(Color::rgb(0.0, 0.0, 0.0).into()))
            }
            _ => None,
        };

        if let Some(material) = material {
            if let Some((thumbnail_e, _)) = q_thumbnail
                .iter()
                .find(|(_, parent_e)| parent_e.0 == node_e)
            {
                info!("Got new thumbnail for {}", node_id);
                commands
                    .entity(thumbnail_e)
                    .remove::<Handle<ColorMaterial>>();
                commands.entity(thumbnail_e).insert(material);
            } else {
                error!("Couldn't find a thumbnail entity for the GUI node");
            }

            *thumb_state = ThumbnailState::Present;
            commands.entity(node_e).remove::<TextureProcessor>();
        }
    }
}

/// Creates a `TextureProcessor` which creates a thumbnail image from the data of a node
/// in a graph. It adds the `TextureProcessor` to the list of thumbnail processors
/// so the result can be retrieved and used in the future.
fn thumbnail_processor(
    tex_pro: &Res<TextureProcessor>,
    node_id: NodeId,
    size: Size,
) -> TextureProcessor {
    let tex_pro_thumb = TextureProcessor::new();

    // Todo: If there's stutter when thumbnails are generated it's probably from here.
    let node_datas = tex_pro.node_slot_data(node_id).unwrap();

    let n_out = tex_pro_thumb
        .add_node(
            Node::new(NodeType::OutputRgba("out".into()))
                .resize_policy(ResizePolicy::SpecificSize(TPSize::new(
                    size.width as u32,
                    size.height as u32,
                )))
                .resize_filter(ResizeFilter::Triangle),
        )
        .unwrap();

    for (i, node_data) in node_datas.iter().take(4).enumerate() {
        if let Ok(end_id) = tex_pro_thumb
            .embed_slot_data_with_id(Arc::clone(node_data), EmbeddedNodeDataId(i as u32))
        {
            let n_node_data = tex_pro_thumb
                .add_node(Node::new(NodeType::Embedded(end_id)))
                .unwrap();

            tex_pro_thumb
                .connect(n_node_data, n_out, SlotId(0), node_data.slot_id)
                .unwrap()
        }
    }

    tex_pro_thumb.process_then_kill();

    info!("Created thumbnail processor for {}", node_id);
    tex_pro_thumb
}

/// Tries to get the output of a given graph.
fn try_get_output(tex_pro: &TextureProcessor) -> Result<Texture, TexProError> {
    let slot_data = tex_pro.engine().try_read()?.slot_datas_output();
    let slot_data = slot_data.first().ok_or(TexProError::InvalidBufferCount)?;

    Ok(Texture::new(
        Extent3d::new(slot_data.size.width as u32, slot_data.size.height as u32, 1),
        TextureDimension::D2,
        slot_data.image.to_rgba(),
        TextureFormat::Rgba8Unorm,
    ))
}
