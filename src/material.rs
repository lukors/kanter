use bevy::prelude::*;
use kanter_core::{node::SlotType, node_graph::NodeId};

use crate::{Hovered, Selected};

pub(crate) struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new().with_system(material.system()),
        );
    }
}

fn material(
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_node: Query<(&Handle<ColorMaterial>, Option<&Hovered>, Option<&Selected>), With<NodeId>>,
    q_slot: Query<(
        &SlotType,
        &Handle<ColorMaterial>,
        Option<&Hovered>,
        Option<&Selected>,
    )>,
) {
    for (material, hovered, selected) in q_node.iter() {
        if let Some(material) = materials.get_mut(material) {
            let value = if selected.is_some() {
                0.25
            } else if hovered.is_some() {
                0.35
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

    for (slot_type, material, hovered, selected) in q_slot.iter() {
        if let Some(material) = materials.get_mut(material) {
            let value = if selected.is_some() {
                1.0
            } else if hovered.is_some() {
                0.8
            } else {
                0.7
            };

            let mut color = match slot_type {
                SlotType::Gray => {
                    let gray_slot = 0.9;
                    Color::rgb(gray_slot, gray_slot, gray_slot)
                }
                SlotType::Rgba => Color::rgb(1.0, 0.8, 0.6),
                SlotType::GrayOrRgba => Color::rgb(0.6, 1.0, 0.8),
            };

            color.set_r(color.r() * value);
            color.set_g(color.g() * value);
            color.set_b(color.b() * value);

            material.color = color;
        }
    }
}
