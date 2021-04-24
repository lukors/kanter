use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

use crate::{Dragged, Hovered, Selected, Slot};

pub(crate) struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new().with_system(material.system()),
        );
    }
}

fn material(
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_node: Query<
        (
            &Handle<ColorMaterial>,
            Option<&Hovered>,
            Option<&Selected>,
            Option<&Dragged>,
        ),
        With<NodeId>,
    >,
    q_slot: Query<
        (
            &Handle<ColorMaterial>,
            Option<&Hovered>,
            Option<&Selected>,
            Option<&Dragged>,
        ),
        With<Slot>,
    >,
) {
    for (material, hovered, selected, dragged) in q_node.iter() {
        if let Some(material) = materials.get_mut(material) {
            let value = if dragged.is_some() {
                0.9
            } else if selected.is_some() {
                0.75
            } else if hovered.is_some() {
                0.6
            } else {
                0.4
            };

            material.color = Color::Rgba {
                red: value,
                green: value,
                blue: value,
                alpha: 1.0,
            };
        }
    }

    for (material, hovered, selected, dragged) in q_slot.iter() {
        if let Some(material) = materials.get_mut(material) {
            let value = if dragged.is_some() {
                0.0
            } else if selected.is_some() {
                0.2
            } else if hovered.is_some() {
                0.5
            } else {
                0.3
            };

            material.color = Color::Rgba {
                red: value,
                green: value,
                blue: value,
                alpha: 1.0,
            };
        }
    }
}
