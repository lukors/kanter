/// Box select tool
use crate::{
    AmbiguitySet, CustomStage, Drag, Draggable, Selected, Slot, ToolState, Workspace,
    CAMERA_DISTANCE, shared::NodeIdComponent, undo::prelude::{UndoCommandManager, Checkpoint}, mouse_interaction::{DeselectAll, SelectNode},
};
use bevy::prelude::*;
#[derive(Component, Default)]
struct BoxSelect {
    start: Vec2,
    end: Vec2,
}
pub(crate) struct BoxSelectPlugin;

impl Plugin for BoxSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(CustomStage::Update)
                .after(CustomStage::Setup)
                .with_system(
                    box_select_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::BoxSelect))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    box_select
                        .system()
                        .with_run_criteria(State::on_update(ToolState::BoxSelect))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    box_select_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::BoxSelect))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        );
    }
}

fn box_select_setup(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.0, 1.0, 0.0, 0.3),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(BoxSelect::default());
}

fn box_select(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    workspace: Res<Workspace>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
    mut q_box_select: Query<(&mut Transform, &mut Sprite, &mut BoxSelect), Without<NodeIdComponent>>,
    q_draggable: Query<
        (Entity, &NodeIdComponent, &GlobalTransform, &Sprite),
        With<Draggable>,
    >,
) {
    if let Ok((mut transform, mut sprite, mut box_select)) = q_box_select.get_single_mut() {
        if workspace.drag == Drag::Starting {
            box_select.start = workspace.cursor_world;
        }
        box_select.end = workspace.cursor_world;
        let box_box = (box_select.start, box_select.end);

        let mut hovered_node_ids = Vec::new();
        
        for (entity, node_id, transform, sprite) in q_draggable.iter() {
            if let Some(size) = sprite.custom_size {
                let size_half = size / 2.0;

                let drag_box = (
                    transform.translation.truncate() - size_half,
                    transform.translation.truncate() + size_half,
                );

                if box_intersect(box_box, drag_box) {
                    hovered_node_ids.push(node_id.0);
                    commands.entity(entity).insert(Selected);
                } else {
                    commands.entity(entity).remove::<Selected>();
                }
            }
        }

        // Todo: I think the ToolState check here is redundant, since this system is set to run on
        // the Update of ToolState::BoxSelect, ToolState::None should be impossible.
        if workspace.drag == Drag::Dropping {
            undo_command_manager.push(Box::new(DeselectAll));
            
            for node_id in hovered_node_ids {
                undo_command_manager.push(Box::new(SelectNode(node_id)));
            }

            undo_command_manager.push(Box::new(Checkpoint));

            tool_state.overwrite_replace(ToolState::None).unwrap();
        }

        // Node intersection

        // for (entity, _node_id, transform, sprite) in q_draggable.iter() {
        //     if let Some(size) = sprite.custom_size {
        //         let size_half = size / 2.0;

        //         let drag_box = (
        //             transform.translation.truncate() - size_half,
        //             transform.translation.truncate() + size_half,
        //         );

        //         if box_intersect(box_box, drag_box) {
        //             commands.entity(entity).insert(Selected);
        //         } else {
        //             commands.entity(entity).remove::<Selected>();
        //         }
        //     }
        // }

        let new_transform = Transform {
            translation: ((box_select.start + box_select.end) / 2.0).extend(CAMERA_DISTANCE),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        *transform = new_transform;

        sprite.custom_size = Some(box_select.start - box_select.end);
    }
}

fn interval_intersect(i_1: (f32, f32), i_2: (f32, f32)) -> bool {
    let i_1 = (i_1.0.min(i_1.1), i_1.0.max(i_1.1));
    let i_2 = (i_2.0.min(i_2.1), i_2.0.max(i_2.1));

    i_1.1 >= i_2.0 && i_2.1 >= i_1.0
}

fn box_intersect(box_1: (Vec2, Vec2), box_2: (Vec2, Vec2)) -> bool {
    let x_1 = (box_1.0.x, box_1.1.x);
    let x_2 = (box_2.0.x, box_2.1.x);
    let y_1 = (box_1.0.y, box_1.1.y);
    let y_2 = (box_2.0.y, box_2.1.y);

    interval_intersect(x_1, x_2) && interval_intersect(y_1, y_2)
}

fn box_select_cleanup(mut commands: Commands, q_box_select: Query<Entity, With<BoxSelect>>) {
    for q_box_select_e in q_box_select.iter() {
        commands.entity(q_box_select_e).despawn();
    }
}
