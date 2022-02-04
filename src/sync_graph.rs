use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use crate::{
    shared::{NodeIdComponent, NodeStateComponent, SlotTypeComponent},
    thumbnail::{Thumbnail, ThumbnailState, THUMBNAIL_SIZE},
    AmbiguitySet, CustomStage, Draggable, Hoverable, Hovered,
};
use bevy::prelude::*;
use vismut_core::{
    edge::Edge as CoreEdge,
    live_graph::{LiveGraph, NodeState},
    node::{node_type::NodeType, Node, Side},
    node_graph::{NodeId, SlotId},
    texture_processor::TextureProcessor,
};
use rand::Rng;

pub const SLOT_SIZE: f32 = 30.;
const SLOT_MARGIN: f32 = 2.;
const SLOT_DISTANCE_X: f32 = THUMBNAIL_SIZE / 2. + SLOT_SIZE / 2. + SLOT_MARGIN;
pub const NODE_SIZE: f32 = THUMBNAIL_SIZE + SLOT_SIZE * 2. + SLOT_MARGIN * 2.;
const SLOT_DISTANCE_Y: f32 = 32. + SLOT_MARGIN;
const SMALLEST_DEPTH_UNIT: f32 = f32::EPSILON * 500.;

trait Name {
    fn title(&self) -> String;
}

impl Name for NodeType {
    fn title(&self) -> String {
        match self {
            Self::CombineRgba => "Combine",
            Self::Image(_) => "Image",
            Self::OutputRgba(_) => "Output",
            Self::SeparateRgba => "Separate",
            Self::Value(_) => "Value",
            _ => "Unnamed",
        }
        .into()
    }
}

// I'm saving the start and end variables for when I want to select the edges themselves.
#[derive(Component, Copy, Clone, Debug)]
pub(crate) struct Edge {
    pub start: Vec2,
    pub end: Vec2,
    pub output_slot: Slot,
    pub input_slot: Slot,
}

impl From<Edge> for CoreEdge {
    fn from(edge: Edge) -> Self {
        Self {
            input_id: edge.input_slot.node_id,
            input_slot: edge.input_slot.slot_id,
            output_id: edge.output_slot.node_id,
            output_slot: edge.output_slot.slot_id,
        }
    }
}

#[derive(Component, Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct Slot {
    pub node_id: NodeId,
    pub side: Side,
    pub slot_id: SlotId,
}

#[derive(Bundle, Default)]
pub(crate) struct GuiNodeBundle {
    #[bundle]
    sprite_bundle: SpriteBundle,
    hoverable: Hoverable,
    hovered: Hovered,
    draggable: Draggable,
    node_id: NodeIdComponent,
    node_state: NodeStateComponent,
    needs_thumbnail: ThumbnailState,
}

#[derive(Bundle, Default)]
pub(crate) struct SlotBundle {
    #[bundle]
    sprite_bundle: SpriteBundle,
    hoverable: Hoverable,
    draggable: Draggable,
    slot: Slot,
    slot_type: SlotTypeComponent,
}

pub(crate) struct SyncGraphPlugin;

impl Plugin for SyncGraphPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(CustomStage::Apply)
                    .after(CustomStage::Update)
                    .with_system(sync_graph.system())
                    .in_ambiguity_set(AmbiguitySet),
            );
    }
}

fn setup(mut commands: Commands, tex_pro: Res<Arc<TextureProcessor>>) {
    let mut live_graph = LiveGraph::new(Arc::clone(&tex_pro.add_buffer_queue));
    live_graph.auto_update = true;
    live_graph.use_cache = true;
    let live_graph = Arc::new(RwLock::new(live_graph));

    tex_pro
        .push_live_graph(Arc::clone(&live_graph))
        .expect("Unable to add graph");

    commands.insert_resource(live_graph);
}

#[allow(clippy::too_many_arguments)]
fn sync_graph(
    mut commands: Commands,
    mut q_node: Query<(
        Entity,
        &NodeIdComponent,
        &mut NodeStateComponent,
        &mut ThumbnailState,
    )>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    let changed_node_ids = live_graph.write().unwrap().changed_consume();

    for node_id in changed_node_ids {
        info!("{:?} changed {{", node_id);

        if let Some((node_gui_e, _, mut node_state, mut thumbnail_state)) = q_node
            .iter_mut()
            .find(|(_, node_id_query, _, _)| node_id_query.0 == node_id)
        {
            if live_graph.read().unwrap().has_node(node_id).is_err() {
                info!("Removing the node");
                commands.entity(node_gui_e).despawn_recursive();
            } else if let Ok(node_state_actual) = live_graph.read().unwrap().node_state(node_id) {
                info!(
                    "State changed from {:?} to {:?}",
                    node_state.0, node_state_actual
                );

                if node_state.0 == NodeState::Clean {
                    // If the node state has been changed in some way--and it used to be
                    // clean--we can't be sure what has happened since then. So we have to
                    // assume that it has been changed.
                    *thumbnail_state = ThumbnailState::Waiting;
                }
                if node_state_actual == NodeState::Clean {
                    *thumbnail_state = ThumbnailState::Missing;
                }
                node_state.0 = node_state_actual;
            } else {
                error!(
                    "Tried updating the state of a node that doesn't exist in the graph: {}",
                    node_id
                );
            }
        }

        info!("}}");
    }
}

pub(crate) fn stretch_between(
    sprite: &mut Sprite,
    transform: &mut Transform,
    start: Vec2,
    end: Vec2,
) {
    let midpoint = start - (start - end) / 2.;
    let distance = start.distance(end);
    let rotation = Vec2::X.angle_between(start - end);

    transform.translation = midpoint.extend(9.0);
    transform.rotation = Quat::from_rotation_z(rotation);
    sprite.custom_size = Some(Vec2::new(distance, 5.));
}

pub fn remove_gui_node(world: &mut World, node_id: NodeId) {
    world
        .get_resource::<Arc<RwLock<LiveGraph>>>()
        .unwrap()
        .write()
        .unwrap()
        .remove_node(node_id)
        .unwrap();
    let (entity, _) = world
        .query::<(Entity, &NodeIdComponent)>()
        .iter(world)
        .find(|(_, node_id_cmp)| node_id == node_id_cmp.0)
        .unwrap();
    despawn_with_children_recursive(world, entity);
}

pub fn spawn_gui_node_2(world: &mut World, node: Node, translation: Vec2) -> Entity {
    world
        .get_resource::<Arc<RwLock<LiveGraph>>>()
        .unwrap()
        .write()
        .unwrap()
        .add_node_with_id(node.clone())
        .unwrap();

    let font = world
        .get_resource::<AssetServer>()
        .unwrap()
        .load("fonts/FiraSans-Regular.ttf");

    let title = node.node_type.title();
    let font_size = SLOT_SIZE;
    let text_y_pos = NODE_SIZE / 2.0 - font_size / 2.0;
    let text_style = TextStyle {
        font,
        font_size,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment {
        horizontal: HorizontalAlign::Center,
        vertical: VerticalAlign::Center,
    };

    let entity = world
        .spawn()
        .insert_bundle(GuiNodeBundle {
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.5, 0.5, 1.0),
                    custom_size: Some(Vec2::new(NODE_SIZE, NODE_SIZE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(Vec3::new(
                    translation.x,
                    translation.y,
                    rand::thread_rng().gen_range(0.0..9.0),
                )),
                ..Default::default()
            },
            node_id: NodeIdComponent(node.node_id),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(Text2dBundle {
                text: Text::with_section(title, text_style, text_alignment),
                transform: Transform::from_translation(Vec3::new(0.0, text_y_pos, 0.0001)),
                ..Default::default()
            });

            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)),
                        ..Default::default()
                    },
                    transform: Transform::from_translation(Vec3::new(0., 0., SMALLEST_DEPTH_UNIT)),
                    ..Default::default()
                })
                .insert(Thumbnail);

            for (i, slot) in node.input_slots().into_iter().enumerate() {
                parent.spawn_bundle(SlotBundle {
                    sprite_bundle: SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.5, 0.5, 0.5),
                            custom_size: Some(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                            ..Default::default()
                        },
                        transform: Transform::from_translation(Vec3::new(
                            -SLOT_DISTANCE_X,
                            THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE_Y * i as f32,
                            SMALLEST_DEPTH_UNIT,
                        )),
                        ..Default::default()
                    },
                    slot: Slot {
                        node_id: node.node_id,
                        side: Side::Input,
                        slot_id: slot.slot_id,
                    },
                    slot_type: SlotTypeComponent(slot.slot_type),
                    ..Default::default()
                });
            }

            for (i, slot) in node.output_slots().into_iter().enumerate() {
                parent.spawn_bundle(SlotBundle {
                    sprite_bundle: SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.5, 0.5, 0.5),
                            custom_size: Some(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                            ..Default::default()
                        },
                        transform: Transform::from_translation(Vec3::new(
                            SLOT_DISTANCE_X,
                            THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE_Y * i as f32,
                            SMALLEST_DEPTH_UNIT,
                        )),
                        ..Default::default()
                    },
                    slot: Slot {
                        node_id: node.node_id,
                        side: Side::Output,
                        slot_id: slot.slot_id,
                    },
                    slot_type: SlotTypeComponent(slot.slot_type),
                    ..Default::default()
                });
            }
        })
        .id();

    entity
}
