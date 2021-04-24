/// Box select tool
use crate::{AmbiguitySet, Stage, Workspace};
use bevy::{prelude::*, window::WindowFocused};

pub(crate) const CAMERA_DISTANCE: f32 = 10.;
pub(crate) struct WorkspaceCameraAnchor;
pub(crate) struct WorkspaceCamera;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub(crate) enum FirstPersonState {
    Off,
    On,
}

impl Default for FirstPersonState {
    fn default() -> Self {
        Self::Off
    }
}

pub(crate) struct Cursor;
pub(crate) struct Crosshair;

pub(crate) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Update)
                .after(Stage::Input)
                .with_system(
                    first_person_on_setup
                        .system()
                        .with_run_criteria(State::on_enter(FirstPersonState::On))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    first_person_on_update
                        .system()
                        .with_run_criteria(State::on_update(FirstPersonState::On))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    first_person_on_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(FirstPersonState::On))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    first_person_off_update
                        .system()
                        .with_run_criteria(State::on_update(FirstPersonState::Off))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    mouse_pan
                        .system()
                        .with_run_criteria(State::on_update(FirstPersonState::Off))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let crosshair_image = asset_server.load("crosshair.png");

    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(WorkspaceCamera)
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Transform::from_translation(Vec3::new(
                    0.,
                    0.,
                    -CAMERA_DISTANCE,
                )))
                .insert(GlobalTransform::default())
                .insert(WorkspaceCameraAnchor)
                .with_children(|parent| {
                    parent
                        .spawn_bundle(SpriteBundle {
                            material: materials.add(crosshair_image.into()),
                            visible: Visible {
                                is_visible: false,
                                is_transparent: true,
                            },
                            ..Default::default()
                        })
                        .insert(Transform::from_translation(Vec3::new(0., 0., 9.0)))
                        .insert(Crosshair);
                });
        });
    commands
        .spawn()
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(Cursor);
}

fn first_person_on_update(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut er_window_focused: EventReader<WindowFocused>,
    mut windows: ResMut<Windows>,
    mut q_camera: Query<(Entity, &mut Transform), With<WorkspaceCamera>>,
    workspace: Res<Workspace>,
) {
    for (_camera_e, mut transform) in q_camera.iter_mut() {
        transform.translation.x += workspace.cursor_delta.x;
        transform.translation.y -= workspace.cursor_delta.y;
    }

    let window = windows.get_primary_mut().unwrap();
    let window_size = Vec2::new(window.width(), window.height());
    window.set_cursor_position(window_size / 2.0);

    if let Some(event_window_focused) = er_window_focused.iter().last() {
        if !event_window_focused.focused && *first_person_state.current() != FirstPersonState::Off {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }
}

fn first_person_off_update(
    mut q_cursor: Query<&mut Transform, With<Cursor>>,
    workspace: Res<Workspace>,
) {
    for mut transform in q_cursor.iter_mut() {
        transform.translation.x = workspace.cursor_world.x;
        transform.translation.y = workspace.cursor_world.y;
    }
}

fn first_person_on_setup(
    mut windows: ResMut<Windows>,
    mut q_camera: Query<Entity, With<WorkspaceCameraAnchor>>,
    mut q_cursor: Query<(Entity, &mut Transform), With<Cursor>>,
    mut q_crosshair: Query<&mut Visible, With<Crosshair>>,
    mut commands: Commands,
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_visibility(false);

    if let Ok(mut crosshair) = q_crosshair.single_mut() {
        crosshair.is_visible = true;
    }

    if let Ok(camera_e) = q_camera.single_mut() {
        if let Ok((cursor_e, mut transform)) = q_cursor.single_mut() {
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            commands.entity(camera_e).push_children(&[cursor_e]);
        }
    }
}

fn first_person_on_cleanup(
    mut windows: ResMut<Windows>,
    mut q_cursor: Query<Entity, With<Cursor>>,
    mut q_crosshair: Query<&mut Visible, With<Crosshair>>,
    mut commands: Commands,
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_visibility(true);

    for mut crosshair in q_crosshair.iter_mut() {
        crosshair.is_visible = false;
    }

    for cursor_e in q_cursor.iter_mut() {
        commands.entity(cursor_e).remove::<Parent>();
    }
}

/// Pan using the mouse.
fn mouse_pan(
    workspace: Res<Workspace>,
    mut camera: Query<&mut Transform, With<WorkspaceCamera>>,
    i_mouse_button: Res<Input<MouseButton>>,
) {
    if i_mouse_button.pressed(MouseButton::Middle) && workspace.cursor_moved {
        if let Ok(mut camera_t) = camera.single_mut() {
            camera_t.translation.x -= workspace.cursor_delta.x;
            camera_t.translation.y += workspace.cursor_delta.y;
        }
    }
}
