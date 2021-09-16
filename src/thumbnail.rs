use crate::{AmbiguitySet, Stage};
use bevy::{
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
};
use kanter_core::{
    error::TexProError,
    live_graph::{LiveGraph, NodeState},
    node::{embed::EmbeddedSlotDataId, node_type::NodeType, Node, ResizeFilter, ResizePolicy},
    node_graph::{NodeId, SlotId},
    slot_data::Size as TPSize,
    texture_processor::TextureProcessor,
};
use std::sync::{Arc, RwLock};

type TexProThumb = (NodeId, TextureProcessor);

pub(crate) const THUMBNAIL_SIZE: f32 = 128.;
pub(crate) struct Thumbnail;

pub(crate) struct ThumbnailPlugin;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum ThumbnailState {
    Waiting,
    Missing,
    Processing,
    Present,
}

impl Default for ThumbnailState {
    fn default() -> Self {
        Self::Waiting
    }
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
                            .chain(thumbnail_state_changed.system())
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn thumbnail_state_changed(
    mut commands: Commands,
    mut q_node: Query<(Entity, &NodeId, &mut ThumbnailState), Changed<ThumbnailState>>,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    tex_pro: Res<Arc<TextureProcessor>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, node_id, mut thumb_state) in q_node
        .iter_mut()
        .filter(|(_, _, state)| **state == ThumbnailState::Missing)
    {
        if let Some(thumb_live_graph) = thumbnail_processor(
            &tex_pro,
            &live_graph,
            *node_id,
            Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
        ) {
            let thumb_live_graph = Arc::new(RwLock::new(thumb_live_graph));
            tex_pro
                .push_live_graph(Arc::clone(&thumb_live_graph))
                .unwrap();
            commands.entity(entity).insert(thumb_live_graph);
            *thumb_state = ThumbnailState::Processing;
        }
        // else if let Some((thumbnail_e, _)) = q_thumbnail
            // .iter()
            // .find(|(_, parent_e)| parent_e.0 == entity)
        // {
            // let material = materials.add(Color::rgb(0.0, 0.0, 0.0).into());
            // commands.entity(thumbnail_e).insert(material);
            // *thumb_state = ThumbnailState::Present;
        // }
    }
}

fn get_thumbnail_loop(
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    mut q_node: Query<(
        Entity,
        &NodeId,
        &mut ThumbnailState,
        &Arc<RwLock<LiveGraph>>,
    )>,
) {
    for (node_e, node_id, mut thumb_state, live_graph) in q_node.iter_mut() {
        let material = match try_get_output(&*live_graph) {
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
            commands.entity(node_e).remove::<Arc<RwLock<LiveGraph>>>();
        }
    }
}

/// Creates a `LiveGraph` that creates a thumbnail image from the data of a node
/// in a graph. It adds the `LiveGraph` to the list of thumbnail processors
/// so the result can be retrieved and used in the future.
fn thumbnail_processor(
    tex_pro: &Res<Arc<TextureProcessor>>,
    live_graph: &Res<Arc<RwLock<LiveGraph>>>,
    node_id: NodeId,
    size: Size,
) -> Option<LiveGraph> {
    dbg!(node_id);
    if let Ok(size) = live_graph.read().unwrap().slot_data_size(node_id, SlotId(0)) {
        dbg!(size);
    }

    if let Ok(slot_data) = live_graph.read().unwrap().slot_data(node_id, SlotId(0)) {
        let mut live_graph_thumb = LiveGraph::new(Arc::clone(&tex_pro.add_buffer_queue));
        let embedded_slot_data_id = live_graph_thumb
            .embed_slot_data_with_id(Arc::clone(slot_data), EmbeddedSlotDataId(0))
            .unwrap();

        let n_embedded = live_graph_thumb
            .add_node(Node::new(NodeType::Embed(embedded_slot_data_id)))
            .unwrap();
        let n_out = live_graph_thumb
            .add_node(
                Node::new(NodeType::OutputRgba("out".into()))
                    .resize_policy(ResizePolicy::SpecificSize(TPSize::new(
                        size.width as u32,
                        size.height as u32,
                    )))
                    .resize_filter(ResizeFilter::Triangle),
            )
            .unwrap();

        live_graph_thumb
            .connect(n_embedded, n_out, SlotId(0), SlotId(0))
            .unwrap();

        live_graph_thumb.auto_update = true;

        info!("Created thumbnail processor for {}", node_id);

        Some(live_graph_thumb)
    } else {
        info!("Failed to create thumbnail processor for {}", node_id);
        None
    }
}

/// Tries to get the first output of a given graph.
fn try_get_output(live_graph: &Arc<RwLock<LiveGraph>>) -> Result<Texture, TexProError> {
    let (output_id, size) = {
        let live_graph = live_graph.read()?;
        let output_id = live_graph.output_ids()[0];
        let size = {
            if live_graph.node_state(output_id)? == NodeState::Clean {
                live_graph.slot_data_size(output_id, SlotId(0))?
            } else {
                return Err(TexProError::NodeDirty);
            }
        };

        (output_id, size)
    };

    Ok(Texture::new(
        Extent3d::new(size.width as u32, size.height as u32, 1),
        TextureDimension::D2,
        LiveGraph::try_buffer_srgba(live_graph, output_id, SlotId(0))?,
        TextureFormat::Rgba8Unorm,
    ))
}
