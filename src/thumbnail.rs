use crate::{AmbiguitySet, Stage, sync_graph::Slot};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    error::TexProError,
    node::{EmbeddedSlotDataId, Node, NodeType, ResizeFilter, ResizePolicy, Side},
    node_graph::{NodeId, SlotId},
    slot_data::Size as TPSize,
    texture_processor::TextureProcessor,
};
use std::sync::{Arc, RwLockReadGuard};

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
        if let Some(thumb_processor) = thumbnail_processor(
            &tex_pro,
            *node_id,
            Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
        ) {
            commands.entity(entity).insert(thumb_processor);
            *thumb_state = ThumbnailState::Processing;
        }
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
                dbg!("OK");
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
) -> Option<TextureProcessor> {
    
    if let Ok(slot_data) = tex_pro.slot_data(node_id, SlotId(0)) {
        let tex_pro_thumb = TextureProcessor::new();
        let embedded_slot_data_id = tex_pro_thumb
            .embed_slot_data_with_id(Arc::clone(&slot_data), EmbeddedSlotDataId(0))
            .unwrap();
    
        let n_embedded = tex_pro_thumb.add_node(Node::new(NodeType::Embedded(embedded_slot_data_id))).unwrap();
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
    
        tex_pro_thumb.connect(n_embedded, n_out, SlotId(0), SlotId(0)).unwrap();
    
        tex_pro_thumb.process_then_kill();
    
        info!("Created thumbnail processor for {}", node_id);

        Some(tex_pro_thumb)
    } else {
        info!("Failed to create thumbnail processor for {}", node_id);
        
        None
    }
}

/// Tries to get the first output of a given graph.
fn try_get_output(tex_pro: &TextureProcessor) -> Result<Texture, TexProError> {
    let output_id = tex_pro.output_ids()[0];
    let slot_data = tex_pro.engine().read()?.slot_data(output_id, SlotId(0))?;
    // dbg!(slot_data.len());
    // let slot_data = slot_data.first().ok_or(TexProError::InvalidBufferCount)?;

    Ok(Texture::new(
        Extent3d::new(slot_data.size.width as u32, slot_data.size.height as u32, 1),
        TextureDimension::D2,
        slot_data.image.to_rgba(),
        TextureFormat::Rgba8Unorm,
    ))
}
