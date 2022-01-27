use bevy::prelude::*;
use kanter_core::node::SlotType;

use crate::{
    mouse_interaction::Active,
    shared::{NodeIdComponent, SlotTypeComponent},
    Hovered, Selected,
};

pub(crate) struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new().with_system(material.system()),
        );
    }
}

// Todo: This function should probably return a color and not a material, if it is needed at all.
// This is due to Bevy 0.6.0, which allows for setting the color of a sprite without a material.
fn material(
    mut q_node: Query<
        (
            &mut Sprite,
            Option<&Hovered>,
            Option<&Selected>,
            Option<&Active>,
        ),
        With<NodeIdComponent>,
    >,
    mut q_slot: Query<
        (
            &SlotTypeComponent,
            &mut Sprite,
            Option<&Hovered>,
            Option<&Selected>,
        ),
        Without<NodeIdComponent>,
    >,
) {
    for (mut sprite, hovered, selected, active) in q_node.iter_mut() {
        let value = if active.is_some() {
            0.0
        } else if selected.is_some() {
            0.25
        } else if hovered.is_some() {
            0.35
        } else {
            0.4
        };

        sprite.color = Color::Rgba {
            red: value,
            green: value,
            blue: value,
            alpha: 1.0,
        };
    }

    for (slot_type, mut sprite, hovered, selected) in q_slot.iter_mut() {
        let value = if selected.is_some() {
            1.0
        } else if hovered.is_some() {
            0.8
        } else {
            0.7
        };

        let mut color = match slot_type.0 {
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

        sprite.color = color;
    }
}
