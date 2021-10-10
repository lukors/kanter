use std::sync::{Arc, RwLock};

use crate::sync_graph::{stretch_between, Edge as GuiEdge, Slot};

use super::{prelude::*, AddRemove};
use bevy::prelude::*;
use kanter_core::{edge::Edge, live_graph::LiveGraph, node::Side};

impl AddRemove for Edge {
    fn add(&self, world: &mut World) {
        if let Some(live_graph) = world.remove_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if let Ok(edge) = live_graph.connect(
                    self.output_id(),
                    self.input_id(),
                    self.output_slot(),
                    self.input_slot(),
                ) {
                    add_gui_edge(world, edge);
                } else {
                    error!("Couldn't add the edge");
                }
            }
            world.insert_resource(live_graph);
        }
    }

    fn remove(&self, world: &mut World) {
        if let Some(live_graph) = world.remove_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                if let Ok(edge) = live_graph.remove_edge(*self) {
                    info!("removed edge: {:?}", &edge);
                    remove_gui_edge(world, edge);
                } else {
                    error!("Couldn't find the edge to remove: {:?}", &self);
                }
            }
            world.insert_resource(live_graph);
        }
    }
}

// fn set_thumbnail_state(world: &mut World, thumbnail_state: ThumbnailState) {
//     // mut q_thumbnail_state: Query<(&NodeId, &mut ThumbnailState), Without<GrabbedEdge>>,

// }

#[derive(Clone, Copy, Debug)]
pub struct RemoveEdge(pub Edge);
impl UndoCommand for RemoveEdge {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.remove(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.add(world);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AddEdge(pub Edge);
impl UndoCommand for AddEdge {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.add(world);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        self.0.remove(world);
    }
}

fn add_gui_edge(world: &mut World, edge: Edge) {
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

    for (slot, global_transform) in world.query::<(&Slot, &GlobalTransform)>().iter(world) {
        if slot.node_id == output_slot.node_id
            && slot.slot_id == output_slot.slot_id
            && slot.side == output_slot.side
        {
            start = global_transform.translation.truncate();
        } else if slot.node_id == input_slot.node_id
            && slot.slot_id == input_slot.slot_id
            && slot.side == input_slot.side
        {
            end = global_transform.translation.truncate();
        }
    }

    let mut sprite = Sprite::new(Vec2::new(5., 5.));
    let mut transform = Transform::default();

    stretch_between(&mut sprite, &mut transform, start, end);
    let mut materials = world.remove_resource::<Assets<ColorMaterial>>().unwrap();
    world
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0., 0., 0.).into()),
            sprite,
            transform,
            ..Default::default()
        })
        .insert(GuiEdge {
            start,
            end,
            output_slot,
            input_slot,
        });
    world.insert_resource(materials);
}

fn remove_gui_edge(world: &mut World, edge: Edge) {
    let mut q_gui_edge = world.query::<(Entity, &GuiEdge)>();

    let edges_to_remove = q_gui_edge
        .iter(world)
        .filter(|(_, gui_edge)| {
            let input_slot = gui_edge.input_slot;
            let output_slot = gui_edge.output_slot;

            input_slot.node_id == edge.input_id
                && input_slot.slot_id == edge.input_slot
                && output_slot.node_id == edge.output_id
                && output_slot.slot_id == edge.output_slot
        })
        .map(|(entity, _)| entity)
        .collect::<Vec<Entity>>();

    for entity in edges_to_remove {
        despawn_with_children_recursive(world, entity);
        info!("removed gui edge: {:?}", entity);
    }
}
