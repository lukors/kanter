/// Box select tool
use crate::{
    AmbiguitySet, Drag, Draggable, Selected, Slot, Stage, ToolState, Workspace, CAMERA_DISTANCE,
};
use bevy::prelude::*;
#[derive(Default)]
struct BoxSelect {
    start: Vec2,
    end: Vec2,
}
pub(crate) struct BoxSelectPlugin;

impl Plugin for BoxSelectPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Update)
                .after(Stage::Input)
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

fn box_select_setup(mut materials: ResMut<Assets<ColorMaterial>>, mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgba(0.0, 1.0, 0.0, 0.3).into()),
            visible: Visible {
                is_visible: true,
                is_transparent: true,
            },
            ..Default::default()
        })
        .insert(BoxSelect::default());
}

fn box_select(
    mut tool_state: ResMut<State<ToolState>>,
    workspace: Res<Workspace>,
    mut q_box_select: Query<(&mut Transform, &mut Sprite, &mut BoxSelect)>,
    q_draggable: Query<
        (Entity, &GlobalTransform, &Sprite),
        (With<Draggable>, Without<BoxSelect>, Without<Slot>),
    >,
    mut commands: Commands,
) {
    if let Ok((mut transform, mut sprite, mut box_select)) = q_box_select.single_mut() {
        if workspace.drag == Drag::Starting {
            box_select.start = workspace.cursor_world;
        }

        if workspace.drag == Drag::Dropping && *tool_state.current() != ToolState::None {
            tool_state.overwrite_replace(ToolState::None).unwrap();
            return;
        }

        box_select.end = workspace.cursor_world;

        let new_transform = Transform {
            translation: ((box_select.start + box_select.end) / 2.0).extend(CAMERA_DISTANCE),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        sprite.size = box_select.start - box_select.end;

        *transform = new_transform;

        // Node intersection
        let box_box = (box_select.start, box_select.end);

        for (entity, transform, sprite) in q_draggable.iter() {
            let size_half = sprite.size / 2.0;

            let drag_box = (
                transform.translation.truncate() - size_half,
                transform.translation.truncate() + size_half,
            );

            if box_intersect(box_box, drag_box) {
                commands.entity(entity).insert(Selected);
            } else {
                commands.entity(entity).remove::<Selected>();
            }
        }
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
