use bevy::prelude::*;

use crate::{workspace::Workspace, AmbiguitySet, Dragged, Stage, ToolState};

#[derive(Default)]
pub(crate) struct Hoverable;
#[derive(Default)]
pub(crate) struct Hovered;

pub(crate) struct HoverablePlugin;

impl Plugin for HoverablePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Update)
                .after(Stage::Input)
                // Other
                .with_system(
                    hoverable
                        .system()
                        .with_run_criteria(State::on_update(ToolState::None))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        );
    }
}

fn hoverable(
    mut commands: Commands,
    workspace: Res<Workspace>,
    q_hoverable: Query<(Entity, &GlobalTransform, &Sprite), (With<Hoverable>, Without<Dragged>)>,
) {
    if workspace.cursor_moved {
        for (entity, global_transform, sprite) in q_hoverable.iter() {
            if box_contains_point(
                global_transform.translation.truncate(),
                sprite.size,
                workspace.cursor_world,
            ) {
                commands.entity(entity).insert(Hovered);
            } else {
                commands.entity(entity).remove::<Hovered>();
            }
        }
    }
}

pub(crate) fn box_contains_point(box_pos: Vec2, box_size: Vec2, point: Vec2) -> bool {
    let half_size = box_size / 2.;

    box_pos.x - half_size.x < point.x
        && box_pos.x + half_size.x > point.x
        && box_pos.y - half_size.y < point.y
        && box_pos.y + half_size.y > point.y
}
