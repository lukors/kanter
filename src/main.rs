#![allow(clippy::type_complexity)]

use bevy::{input::mouse::MouseMotion, prelude::*, render::camera::Camera, window::WindowFocused};

const MODE: &str = "mode";

fn main() {
    App::build()
        .init_resource::<StateGlobal>()
        .add_resource(WindowDescriptor {
            title: "Bevy".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: true,
            ..Default::default()
        })
        .add_resource(State::new(ModeState::None))
        .add_stage_before(stage::UPDATE, MODE, StateStage::<ModeState>::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(KanterPlugin)
        .run();
}

#[derive(Clone, Copy, PartialEq)]
enum ModeState {
    None,
    BoxSelect,
}

impl Default for ModeState {
    fn default() -> Self {
        Self::None
    }
}

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(ActiveWorkspace(None))
            .add_startup_system(setup.system())
            .add_system_to_stage(stage::PRE_UPDATE, workspace.system())
            .add_system_to_stage(stage::PRE_UPDATE, mode.system())
            .on_state_enter(MODE, ModeState::BoxSelect, box_select_setup.system())
            .on_state_update(MODE, ModeState::BoxSelect, box_select.system())
            .on_state_exit(MODE, ModeState::BoxSelect, box_select_cleanup.system())
            .add_system_to_stage(stage::UPDATE, camera.system())
            .add_system_to_stage(stage::UPDATE, cursor_transform.system())
            .add_system_to_stage(stage::UPDATE, draggable.system())
            .add_system_to_stage(stage::UPDATE, hoverable.system())
            .add_system_to_stage(stage::UPDATE, first_person.system())
            .add_system_to_stage(stage::POST_UPDATE, drag.system())
            .add_system_to_stage(stage::POST_UPDATE, drop.system())
            .add_system_to_stage(stage::POST_UPDATE, cursor_visibility.system())
            .add_system_to_stage(stage::POST_UPDATE, crosshair_visibility.system())
            .add_system_to_stage(stage::POST_UPDATE, material.system());
    }
}

const NODE_SIZE: f32 = 128.;
const NODE_SIZE_VEC: Vec2 = Vec2 {
    x: NODE_SIZE,
    y: NODE_SIZE,
};

fn setup(
    mut r_aw: ResMut<ActiveWorkspace>,
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let test_image = asset_server.load("image_2.png");
    let crosshair_image = asset_server.load("crosshair.png");

    r_aw.0 = commands
        .spawn((Workspace::default(), FirstPerson(false)))
        .current_entity();
    let r_aw_e = r_aw.0.unwrap();

    commands
        .spawn(Camera2dBundle::default())
        .with(Owner(r_aw_e))
        .with_children(|parent| {
            parent
                .spawn(SpriteBundle {
                    material: materials.add(crosshair_image.into()),
                    ..Default::default()
                })
                .with(Crosshair)
                .with(Owner(r_aw_e));
        })
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Cursor,
            Owner(r_aw_e),
        ));

    for _ in 0..4 {
        commands
            .spawn(SpriteBundle {
                material: materials.add(test_image.clone().into()),
                sprite: Sprite::new(NODE_SIZE_VEC),
                ..Default::default()
            })
            .with(Hoverable)
            .with(Draggable)
            .with(Owner(r_aw_e));
    }
}

// TODO: Break out stuff from Workspace into individual components if they don't need to be grouped.
//       This allows for more fine grained control over when systems run, resulting in cleaner code.
//       At least first_person can be broken out (it has been broken out now).
// TODO: Add a camera entity component to workspace so its more reliable to get to.
// TODO: Parent everything to the workspace it belongs to, so everything automatically is removed
//       when the workspace is.
// TODO: Box select
// TODO: Fix bug where first person selection aim is off after dropping a node outside the window,
//       can debug this by using box select in first person mode.
// TODO: Handle first person mode with states.
// TODO: Make cursor data global and not per workspace.
// TODO: Remove owner concept to simplify the code, can add back later if I want it.
// TODO: Remove as many unwraps as possible to reduce risk of crashes.
// TODO: Test box select with OS level DPI scaling on.
// TODO: Have easy global access to re-used textures.
// TODO: Implement ablitiy to select nodes, then make dragging affect selected nodes,
//       then implement box select.

struct ActiveWorkspace(Option<Entity>);
struct Owner(Entity);

#[derive(Default)]
struct Workspace {
    cursor_screen: Vec2,
    cursor_world: Vec2,
    cursor_delta: Vec2,
    cursor_moved: bool,
}

struct FirstPerson(bool);

struct Cursor;

struct Draggable;
struct Dragged;
struct Dropped;

struct Hoverable;
struct Hovered;

struct BoxSelectCursor;

#[derive(Default)]
struct BoxSelect {
    start: Vec2,
    end: Vec2,
}

fn workspace(
    mut state_global: ResMut<StateGlobal>,
    e_cursor_moved: Res<Events<CursorMoved>>,
    e_mouse_motion: Res<Events<MouseMotion>>,
    windows: Res<Windows>,
    mut q_workspace: Query<&mut Workspace>,
    q_camera: Query<&Transform, With<Camera>>,
) {
    let mut event_cursor_delta: Vec2 = Vec2::zero();
    for event_motion in state_global.er_mouse_motion.iter(&e_mouse_motion) {
        event_cursor_delta += event_motion.delta;
    }
    let event_cursor_screen = state_global.er_cursor_moved.latest(&e_cursor_moved);

    for mut workspace in q_workspace.iter_mut() {
        if let Some(event_cursor_screen) = event_cursor_screen {
            workspace.cursor_screen = event_cursor_screen.position;

            let window = windows.get_primary().unwrap();
            let cam_transform = q_camera.iter().last().unwrap();
            workspace.cursor_world =
                cursor_to_world(window, cam_transform, event_cursor_screen.position);

            workspace.cursor_moved = true;
        } else {
            workspace.cursor_moved = false;
        }

        workspace.cursor_delta = event_cursor_delta;
    }
}

fn mode(
    mut mode: ResMut<State<ModeState>>,
    input: Res<Input<KeyCode>>,
) {
    let mode_current = *mode.current();

    if input.just_pressed(KeyCode::Escape) && mode_current != ModeState::None {
        mode.set_next(ModeState::None).unwrap();
    }

    if input.just_pressed(KeyCode::B) && mode_current != ModeState::BoxSelect {
        mode.set_next(ModeState::BoxSelect).unwrap();
    }
}

fn box_select_setup(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    commands: &mut Commands,
    q_workspace: Query<&Workspace>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    let box_image = asset_server.load("box_select.png");
    let crosshair_image = asset_server.load("crosshair.png");

    let box_material = materials.add(box_image.into());
    materials.get_mut(&box_material).unwrap().color.set_a(0.25);

    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(workspace.cursor_world.extend(0.)),
        material: materials.add(crosshair_image.into()),
        ..Default::default()
    })
    .with(BoxSelectCursor)
    .spawn(SpriteBundle {
        material: box_material,
        visible: Visible { is_visible: false, is_transparent: true },
        ..Default::default()
    })
    .with(BoxSelect::default());
}

fn box_select(
    i_mouse_button: Res<Input<MouseButton>>,
    mut mode: ResMut<State<ModeState>>,
    q_workspace: Query<&Workspace>,
    mut q_box_select_cursor: Query<&mut Transform, With<BoxSelectCursor>>,
    mut q_box_select: Query<(&mut Transform, &mut Visible, &Sprite, &mut BoxSelect)>,
) {
    let workspace = q_workspace.iter().next().unwrap();


    for mut transform in q_box_select_cursor.iter_mut() {
        transform.translation = workspace.cursor_world.extend(0.);
    }

    for (mut transform, mut visible, sprite, mut box_select) in q_box_select.iter_mut() {
        if i_mouse_button.just_pressed(MouseButton::Left) {
            visible.is_visible = true;
            box_select.start = workspace.cursor_world;
        }

        if i_mouse_button.just_released(MouseButton::Left)
        && visible.is_visible
        && *mode.current() != ModeState::None {
            mode.overwrite_next(ModeState::None).unwrap();
            return;
        }

        if visible.is_visible {
            box_select.end = box_select.start - workspace.cursor_world;

            let new_transform = Transform {
                translation: (box_select.start - box_select.end / 2.0).extend(0.),
                rotation: Quat::identity(),
                scale: (box_select.end / sprite.size).extend(1.0),
            };

            *transform = new_transform;
        }
    }
}

fn box_select_cleanup(
    commands: &mut Commands,
    q_box_select_cursor: Query<Entity, With<BoxSelectCursor>>,
    q_box_select: Query<Entity, With<BoxSelect>>,
) {
    for box_select_cursor_e in q_box_select_cursor.iter() {
        commands.despawn(box_select_cursor_e);
    }

    for q_box_select_e in q_box_select.iter() {
        commands.despawn(q_box_select_e);
    }
}

fn cursor_transform(
    r_aw: Res<ActiveWorkspace>,
    commands: &mut Commands,
    q_workspace: Query<(Entity, &Workspace, &FirstPerson)>,
    q_camera: Query<(Entity, &Owner), With<Camera>>,
    mut q_cursor: Query<(Entity, &Owner, &mut Transform), With<Cursor>>,
) {
    for (workspace_e, workspace, first_person) in q_workspace.iter() {
        if !workspace_matches(&r_aw, workspace_e) {
            continue;
        }

        if first_person.0 {
            for (camera_e, owner) in q_camera.iter() {
                if !owner_matches(&r_aw, owner) {
                    continue;
                }

                for (cursor_e, owner, mut transform) in q_cursor.iter_mut() {
                    if !owner_matches(&r_aw, owner) {
                        continue;
                    }

                    transform.translation.x = 0.;
                    transform.translation.y = 0.;
                    commands.insert_one(cursor_e, Parent(camera_e));
                }
            }
        } else {
            for (cursor_e, owner, mut transform) in q_cursor.iter_mut() {
                if !owner_matches(&r_aw, owner) {
                    continue;
                }

                transform.translation.x = workspace.cursor_world.x;
                transform.translation.y = workspace.cursor_world.y;
                commands.remove_one::<Parent>(cursor_e);
            }
        }
    }
}

fn hoverable(
    commands: &mut Commands,
    q_workspace: Query<&Workspace>,
    q_hoverable: Query<(Entity, &Transform, &Sprite), (With<Hoverable>, Without<Dragged>)>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    if workspace.cursor_moved {
        for (entity, transform, sprite) in q_hoverable.iter() {
            let half_width = sprite.size.x / 2.;
            let half_height = sprite.size.y / 2.;

            if transform.translation.x - half_width < workspace.cursor_world.x
                && transform.translation.x + half_width > workspace.cursor_world.x
                && transform.translation.y - half_height < workspace.cursor_world.y
                && transform.translation.y + half_height > workspace.cursor_world.y
            {
                commands.insert_one(entity, Hovered);
            } else {
                commands.remove_one::<Hovered>(entity);
            }
        }
    }
}

fn material(
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_hoverable: Query<
        (&Handle<ColorMaterial>, Option<&Hovered>, Option<&Dragged>),
        With<Hoverable>,
    >,
) {
    let mut first = true;

    for (material, hovered, dragged) in q_hoverable.iter() {
        let (red, green, alpha) = if dragged.is_some() {
            (0.0, 1.0, 1.0)
        } else if first && hovered.is_some() {
            first = false;
            (1.0, 0.0, 1.0)
        } else if hovered.is_some() {
            (1.0, 1.0, 0.5)
        } else {
            (1.0, 1.0, 1.0)
        };

        materials.get_mut(material).unwrap().color.set_r(red);
        materials.get_mut(material).unwrap().color.set_g(green);
        materials.get_mut(material).unwrap().color.set_a(alpha);
    }
}

fn cursor_to_world(window: &Window, cam_transform: &Transform, cursor_pos: Vec2) -> Vec2 {
    // get the size of the window
    let size = Vec2::new(window.width() as f32, window.height() as f32);

    // the default orthographic projection is in pixels from the center;
    // just undo the translation
    let screen_pos = cursor_pos - size / 2.0;

    // apply the camera transform
    let out = cam_transform.compute_matrix() * screen_pos.extend(0.0).extend(1.0);
    Vec2::new(out.x, out.y)
}

fn draggable(
    commands: &mut Commands,
    i_mouse_button: Res<Input<MouseButton>>,
    q_pressed: Query<Entity, (With<Hovered>, With<Draggable>)>,
    q_released: Query<Entity, With<Dragged>>,
) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        if let Some(entity) = q_pressed.iter().next() {
            commands.insert_one(entity, Dragged);
        }
    } else if i_mouse_button.just_released(MouseButton::Left) {
        for entity in q_released.iter() {
            commands.remove_one::<Dragged>(entity);

            commands.insert_one(entity, Dropped);
        }
    }
}

fn drag(
    commands: &mut Commands,
    mut q_dragged: Query<(Entity, &mut Transform, &GlobalTransform), Added<Dragged>>,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
) {
    if let Some((cursor_e, cursor_transform)) = q_cursor.iter().next() {
        for (entity, mut transform, global_transform) in q_dragged.iter_mut() {
            let global_pos = global_transform.translation - cursor_transform.translation;

            commands.insert_one(entity, Parent(cursor_e));

            transform.translation.x = global_pos.x;
            transform.translation.y = global_pos.y;
        }
    }
}

fn drop(
    commands: &mut Commands,
    mut q_dropped: Query<(Entity, &mut Transform, &GlobalTransform), Added<Dropped>>,
) {
    for (entity, mut transform, global_transform) in q_dropped.iter_mut() {
        let global_pos = global_transform.translation;

        transform.translation.x = global_pos.x;
        transform.translation.y = global_pos.y;

        commands.remove_one::<Parent>(entity);
        commands.remove_one::<Dropped>(entity);
    }
}

struct Crosshair;

#[derive(Default)]
struct StateGlobal {
    er_mouse_motion: EventReader<MouseMotion>,
    er_cursor_moved: EventReader<CursorMoved>,
    er_window_focused: EventReader<WindowFocused>,
}

fn camera(
    r_aw: Res<ActiveWorkspace>,
    mut q_camera: Query<(&Owner, &mut Transform), With<Camera>>,
    q_workspace: Query<(Entity, &Workspace, &FirstPerson)>,
) {
    for (workspace_e, workspace, first_person) in q_workspace.iter() {
        if !workspace_matches(&r_aw, workspace_e) {
            continue;
        }

        if !first_person.0 {
            return;
        }

        for (owner, mut transform) in q_camera.iter_mut() {
            if !owner_matches(&r_aw, owner) {
                continue;
            }

            transform.translation.x += workspace.cursor_delta.x;
            transform.translation.y -= workspace.cursor_delta.y;
        }
    }
}

fn owner_matches(r_aw: &Res<ActiveWorkspace>, owner: &Owner) -> bool {
    workspace_matches(r_aw, owner.0)
}

fn workspace_matches(r_aw: &Res<ActiveWorkspace>, entity: Entity) -> bool {
    if let Some(r_aw_e) = r_aw.0 {
        entity == r_aw_e
    } else {
        false
    }
}

fn cursor_visibility(
    r_aw: Res<ActiveWorkspace>,
    mut windows: ResMut<Windows>,
    q_workspace: Query<(Entity, &FirstPerson), Changed<FirstPerson>>,
) {
    for (workspace_e, first_person) in q_workspace.iter() {
        if !workspace_matches(&r_aw, workspace_e) {
            continue;
        }

        let window = windows.get_primary_mut().unwrap();
        window.set_cursor_visibility(!first_person.0);

        let window_size = Vec2::new(window.width(), window.height());
        if first_person.0 {
            window.set_cursor_position(window_size / 2.0);
        }
    }
}

fn crosshair_visibility(
    r_aw: Res<ActiveWorkspace>,
    q_workspace: Query<(Entity, &FirstPerson)>,
    mut query: Query<(&Owner, &mut Visible), With<Crosshair>>,
) {
    for (workspace_e, first_person) in q_workspace.iter() {
        if !workspace_matches(&r_aw, workspace_e) {
            continue;
        }

        for (owner, mut visible) in query.iter_mut() {
            if !owner_matches(&r_aw, owner) {
                continue;
            }

            visible.is_visible = first_person.0;
        }
    }
}

fn first_person(
    r_aw: Res<ActiveWorkspace>,
    input: Res<Input<KeyCode>>,
    e_window_focused: Res<Events<WindowFocused>>,
    mut state_global: ResMut<StateGlobal>,
    mut q_workspace: Query<(Entity, &mut FirstPerson)>,
) {
    for (workspace_e, mut first_person) in q_workspace.iter_mut() {
        if !workspace_matches(&r_aw, workspace_e) {
            continue;
        }

        if input.just_pressed(KeyCode::Tab) {
            first_person.0 = !first_person.0;
        }
        if input.just_pressed(KeyCode::Escape) {
            first_person.0 = false;
        }

        let event_window_focused = state_global.er_window_focused.latest(&e_window_focused);
        if let Some(event_window_focused) = event_window_focused {
            if !event_window_focused.focused {
                first_person.0 = false;
            }
        }
    }
}
