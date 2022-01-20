use std::sync::{Arc, RwLock};

use crate::{
    thumbnail::{Thumbnail, ThumbnailState, THUMBNAIL_SIZE},
    AmbiguitySet, Draggable, Hoverable, Hovered, Stage, shared::{NodeIdComponent, NodeStateComponent, SlotTypeComponent},
};
use bevy::prelude::*;
use kanter_core::{
    live_graph::{LiveGraph, NodeState},
    node::{Node, Side, SlotType},
    node_graph::{NodeId, SlotId},
    texture_processor::TextureProcessor,
};
use rand::Rng;

pub const SLOT_SIZE: f32 = 30.;
const SLOT_MARGIN: f32 = 2.;
const SLOT_DISTANCE_X: f32 = THUMBNAIL_SIZE / 2. + SLOT_SIZE / 2. + SLOT_MARGIN;
const NODE_SIZE: f32 = THUMBNAIL_SIZE + SLOT_SIZE * 2. + SLOT_MARGIN * 2.;
const SLOT_DISTANCE_Y: f32 = 32. + SLOT_MARGIN;
const SMALLEST_DEPTH_UNIT: f32 = f32::EPSILON * 500.;

// I'm saving the start and end variables for when I want to select the edges themselves.
#[derive(Component, Copy, Clone, Debug)]
pub(crate) struct Edge {
    pub start: Vec2,
    pub end: Vec2,
    pub output_slot: Slot,
    pub input_slot: Slot,
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
    // selected: Selected,
    draggable: Draggable,
    // dragged: Dragged,
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
        app.add_startup_system(setup.system());
            // .add_system_set_to_stage(
            //     CoreStage::Update,
            //     SystemSet::new()
            //         .label(Stage::Apply)
            //         .after(Stage::Update)
            //         .with_system(sync_graph.system())
            //         .in_ambiguity_set(AmbiguitySet),
            // );
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
    // mut materials: ResMut<Assets<ColorMaterial>>,
    mut q_node: Query<(Entity, &NodeIdComponent, &mut NodeStateComponent, &mut ThumbnailState)>,
    // q_edge: Query<(Entity, &Edge)>,
    // q_slot: Query<(&Slot, &GlobalTransform)>,
    // q_selected: Query<Entity, With<Selected>>,
    // workspace: Res<Workspace>,
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

            // info!("Updating visual edges");

            // Removing edges for the node so they can be re-created in the next step.
            // for (entity, _) in q_edge.iter().filter(|(_, edge)| {
            //     edge.input_slot.node_id == node_id || edge.output_slot.node_id == node_id
            // }) {
            //     commands.entity(entity).despawn_recursive();
            // }

            // // Adding the current edges.
            // for edge in live_graph
            //     .read()
            //     .unwrap()
            //     .edges()
            //     .iter()
            //     .filter(|edge| edge.input_id == node_id)
            // {
            //     let output_slot = Slot {
            //         node_id: edge.output_id,
            //         side: Side::Output,
            //         slot_id: edge.output_slot,
            //     };
            //     let input_slot = Slot {
            //         node_id: edge.input_id,
            //         side: Side::Input,
            //         slot_id: edge.input_slot,
            //     };
            //     let mut start = Vec2::ZERO;
            //     let mut end = Vec2::ZERO;

            //     for (slot, slot_t) in q_slot.iter() {
            //         if slot.node_id == output_slot.node_id
            //             && slot.slot_id == output_slot.slot_id
            //             && slot.side == output_slot.side
            //         {
            //             start = slot_t.translation.truncate();
            //         } else if slot.node_id == input_slot.node_id
            //             && slot.slot_id == input_slot.slot_id
            //             && slot.side == input_slot.side
            //         {
            //             end = slot_t.translation.truncate();
            //         }
            //     }

            //     let mut sprite = Sprite::new(Vec2::new(5., 5.));
            //     let mut transform = Transform::default();

            //     stretch_between(&mut sprite, &mut transform, start, end);

            //     commands
            //         .spawn_bundle(SpriteBundle {
            //             material: materials.add(Color::rgb(0., 0., 0.).into()),
            //             sprite,
            //             transform,
            //             ..Default::default()
            //         })
            //         .insert(Edge {
            //             start,
            //             end,
            //             output_slot,
            //             input_slot,
            //         });
            // }
        } else {
            // info!("Adding the node");

            // Deselect everything so the new node(s) can be selected instead.
            // for entity in q_selected.iter() {
            //     commands.entity(entity).remove::<Selected>();
            // }

            // let node = live_graph.read().unwrap().node(node_id).unwrap();
            // spawn_gui_node(&mut commands, &mut materials, &node, workspace.cursor_world);
        }

        info!("}}");
    }
}

pub fn spawn_gui_node(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    node: &Node,
    position: Vec2,
) {
    commands
        .spawn_bundle(GuiNodeBundle {
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.5, 0.5, 1.0),
                    custom_size: Some(Vec2::new(NODE_SIZE, NODE_SIZE)),
                    ..Default::default()
                },
                transform: Transform::from_translation(Vec3::new(
                    position.x,
                    position.y,
                    rand::thread_rng().gen_range(0.0..9.0),
                )),
                ..Default::default()
            },
            node_id: NodeIdComponent(node.node_id),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLACK,
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
        });
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

    let mut materials = world.remove_resource::<Assets<ColorMaterial>>().unwrap();
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
            parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLACK,
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

    world.insert_resource(materials);
    entity
}
