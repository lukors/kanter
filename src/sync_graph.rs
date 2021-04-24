use std::sync::Arc;

use crate::{
    workspace::Workspace, AmbiguitySet, Draggable, Dragged, Hoverable, Hovered, Selected, Stage,
    Thumbnail, THUMBNAIL_SIZE,
};
use bevy::prelude::*;
use kanter_core::{
    dag::TextureProcessor,
    node::{Node, NodeType, Side},
    node_graph::{NodeId, SlotId},
};
use rand::Rng;

const SLOT_SIZE: f32 = 30.;
const SLOT_MARGIN: f32 = 2.;
const SLOT_DISTANCE_X: f32 = THUMBNAIL_SIZE / 2. + SLOT_SIZE / 2. + SLOT_MARGIN;
const NODE_SIZE: f32 = THUMBNAIL_SIZE + SLOT_SIZE * 2. + SLOT_MARGIN * 2.;
const SLOT_DISTANCE_Y: f32 = 32. + SLOT_MARGIN;
const SMALLEST_DEPTH_UNIT: f32 = f32::EPSILON * 500.;

// I'm saving the start and end variables for when I want to select the edges themselves.
pub(crate) struct Edge {
    pub start: Vec2,
    pub end: Vec2,
    pub output_slot: Slot,
    pub input_slot: Slot,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Slot {
    pub node_id: NodeId,
    pub side: Side,
    pub slot_id: SlotId,
}

pub(crate) struct SyncGraphPlugin;

impl Plugin for SyncGraphPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_non_send_resource(TextureProcessor::new())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(sync_graph.system())
                    .in_ambiguity_set(AmbiguitySet),
            );
    }
}

fn sync_graph(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_node_id: Query<(Entity, &NodeId)>,
    q_edge: Query<Entity, With<Edge>>,
    q_slot: Query<(&Slot, &GlobalTransform)>,
    tex_pro: Res<TextureProcessor>,
    workspace: Res<Workspace>,
) {
    if tex_pro.is_changed() {
        let tp_node_ids = tex_pro.node_graph.node_ids();
        let existing_gui_node_ids: Vec<NodeId> =
            q_node_id.iter().map(|(_, node_id)| *node_id).collect();
        let new_ids: Vec<NodeId> = tp_node_ids
            .iter()
            .filter(|tp_node_id| !existing_gui_node_ids.contains(tp_node_id))
            .copied()
            .collect();
        let removed_ids: Vec<NodeId> = existing_gui_node_ids
            .iter()
            .filter(|gui_node_id| !tp_node_ids.contains(gui_node_id))
            .copied()
            .collect();

        // Create gui nodes for any new nodes in the graph.
        for node_id in new_ids {
            let node = tex_pro.node_graph.node_with_id(node_id).unwrap();
            spawn_gui_node(&mut commands, &mut materials, &node, workspace.cursor_world);
        }

        // Remove the gui nodes for any nodes that don't exist in the graph.
        for (node_e, _) in q_node_id
            .iter()
            .filter(|(_, node_id)| removed_ids.contains(node_id))
        {
            commands.entity(node_e).despawn_recursive();
        }

        // Remove all edges so we can create new ones. This should be optimized to move
        // existing edges.
        for edge_e in q_edge.iter() {
            commands.entity(edge_e).despawn_recursive();
        }

        for edge in tex_pro.node_graph.edges.iter() {
            let output_slot = Slot {
                node_id: edge.output_id,
                side: Side::Output,
                slot_id: edge.output_slot,
            };
            let input_slot = Slot {
                node_id: edge.input_id,
                side: Side::Input,
                slot_id: edge.input_slot,
            };
            let mut start = Vec2::ZERO;
            let mut end = Vec2::ZERO;

            for (slot, slot_t) in q_slot.iter() {
                if slot.node_id == output_slot.node_id
                    && slot.slot_id == output_slot.slot_id
                    && slot.side == output_slot.side
                {
                    start = slot_t.translation.truncate();
                } else if slot.node_id == input_slot.node_id
                    && slot.slot_id == input_slot.slot_id
                    && slot.side == input_slot.side
                {
                    end = slot_t.translation.truncate();
                }
            }

            let mut sprite = Sprite::new(Vec2::new(5., 5.));
            let mut transform = Transform::default();

            stretch_between(&mut sprite, &mut transform, start, end);

            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                    sprite,
                    transform,
                    ..Default::default()
                })
                .insert(Edge {
                    input_slot,
                    output_slot,
                    start,
                    end,
                });
        }
    }
}

fn spawn_gui_node(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    node: &Arc<Node>,
    position: Vec2,
) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            sprite: Sprite::new(Vec2::new(NODE_SIZE, NODE_SIZE)),
            transform: Transform::from_translation(Vec3::new(
                position.x,
                position.y,
                rand::thread_rng().gen_range(0.0..9.0),
            )),
            ..Default::default()
        })
        .insert(Hoverable)
        .insert(Hovered)
        .insert(Selected)
        .insert(Draggable)
        .insert(Dragged)
        .insert(node.node_id)
        .with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
                    sprite: Sprite::new(Vec2::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)),
                    transform: Transform::from_translation(Vec3::new(0., 0., SMALLEST_DEPTH_UNIT)),
                    ..Default::default()
                })
                .insert(Thumbnail);
            for i in 0..node.capacity(Side::Input) {
                parent
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
                        sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        transform: Transform::from_translation(Vec3::new(
                            -SLOT_DISTANCE_X,
                            THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE_Y * i as f32,
                            SMALLEST_DEPTH_UNIT,
                        )),
                        ..Default::default()
                    })
                    .insert(Hoverable)
                    .insert(Draggable)
                    .insert(Slot {
                        node_id: node.node_id,
                        side: Side::Input,
                        slot_id: SlotId(i as u32),
                    })
                    .id();
            }

            for i in 0..node.capacity(Side::Output) {
                if node.node_type == NodeType::OutputRgba || node.node_type == NodeType::OutputGray
                {
                    break;
                }
                parent
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
                        sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        transform: Transform::from_translation(Vec3::new(
                            SLOT_DISTANCE_X,
                            THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE_Y * i as f32,
                            SMALLEST_DEPTH_UNIT,
                        )),
                        ..Default::default()
                    })
                    .insert(Hoverable)
                    .insert(Draggable)
                    .insert(Slot {
                        node_id: node.node_id,
                        side: Side::Output,
                        slot_id: SlotId(i as u32),
                    })
                    .id();
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

    transform.translation = midpoint.extend(0.0);
    transform.rotation = Quat::from_rotation_z(rotation);
    sprite.size = Vec2::new(distance, 5.);
}
