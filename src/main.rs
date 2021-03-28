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
    Grab,
}

impl Default for ModeState {
    fn default() -> Self {
        Self::None
    }
}

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_to_stage(stage::PRE_UPDATE, workspace.system())
            .add_system_to_stage(stage::PRE_UPDATE, mode.system())
            .on_state_enter(MODE, ModeState::BoxSelect, box_select_setup.system())
            .on_state_update(MODE, ModeState::BoxSelect, box_select.system())
            .on_state_exit(MODE, ModeState::BoxSelect, box_select_cleanup.system())
            .on_state_enter(MODE, ModeState::Grab, grab_setup.system())
            .on_state_update(MODE, ModeState::Grab, grab.system())
            .on_state_exit(MODE, ModeState::Grab, grab_cleanup.system())
            .on_state_exit(MODE, ModeState::None, none_setup.system())
            .on_state_update(MODE, ModeState::None, select_single.system())
            .on_state_update(MODE, ModeState::None, draggable.system())
            .on_state_update(MODE, ModeState::None, hoverable.system())
            .add_system_to_stage(stage::UPDATE, camera.system())
            .add_system_to_stage(stage::UPDATE, cursor_transform.system())
            .add_system_to_stage(stage::UPDATE, first_person.system())
            .add_system_to_stage(stage::UPDATE, deselect.system())
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
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let test_image = asset_server.load("image_2.png");
    let crosshair_image = asset_server.load("crosshair.png");

    commands
        .spawn((Workspace::default(), FirstPerson(false)))
        .spawn(Camera2dBundle::default())
        .with_children(|parent| {
            parent
                .spawn(SpriteBundle {
                    material: materials.add(crosshair_image.into()),
                    ..Default::default()
                })
                .with(Crosshair);
        })
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            Cursor,
        ));

    for _ in 0..4 {
        commands
            .spawn(SpriteBundle {
                material: materials.add(test_image.clone().into()),
                sprite: Sprite::new(NODE_SIZE_VEC),
                ..Default::default()
            })
            .with(Hoverable)
            .with(Draggable);
    }
}

// TODO: Handle first person mode with states.
// TODO: Make cursor data global and not per workspace.
// TODO: Remove as many unwraps as possible to reduce risk of crashes.
// TODO: Have easy global access to re-used textures.
// TODO: Try removing `Dropped` component and instead check for the deletion of `Dragged` component.
// TODO: Save state before grabbing and restore it if escape is pressed, undo system?
// TODO: Stop drag and drop if escape is pressed.

#[derive(Default)]
struct Workspace {
    cursor_screen: Vec2,
    cursor_world: Vec2,
    cursor_delta: Vec2,
    cursor_moved: bool,
}

struct FirstPerson(bool);

struct Cursor;

struct Hoverable;
struct Hovered;

struct Selected;

struct Draggable;
struct Dragged;
struct Dropped;

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

fn mode(mut mode: ResMut<State<ModeState>>, input: Res<Input<KeyCode>>) {
    let mode_current = *mode.current();

    if input.just_pressed(KeyCode::Escape) && mode_current != ModeState::None {
        mode.set_next(ModeState::None).unwrap();
    }

    if input.just_pressed(KeyCode::B) && mode_current != ModeState::BoxSelect {
        mode.set_next(ModeState::BoxSelect).unwrap();
    }

    if input.just_pressed(KeyCode::G) && mode_current != ModeState::Grab {
        mode.set_next(ModeState::Grab).unwrap();
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

    commands
        .spawn(SpriteBundle {
            transform: Transform::from_translation(workspace.cursor_world.extend(0.)),
            material: materials.add(crosshair_image.into()),
            ..Default::default()
        })
        .with(BoxSelectCursor)
        .spawn(SpriteBundle {
            material: box_material,
            visible: Visible {
                is_visible: false,
                is_transparent: true,
            },
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
    q_draggable: Query<(Entity, &Transform, &Sprite), With<Draggable>>,
    commands: &mut Commands,
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
            && *mode.current() != ModeState::None
        {
            mode.overwrite_next(ModeState::None).unwrap();
            return;
        }

        if visible.is_visible {
            box_select.end = workspace.cursor_world;

            let new_transform = Transform {
                translation: ((box_select.start + box_select.end) / 2.0).extend(0.),
                rotation: Quat::identity(),
                scale: ((box_select.start - box_select.end) / sprite.size).extend(1.0),
            };

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
                    commands.insert_one(entity, Selected);
                } else {
                    commands.remove_one::<Selected>(entity);
                }
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
    commands: &mut Commands,
    q_workspace: Query<(&Workspace, &FirstPerson)>,
    q_camera: Query<Entity, With<Camera>>,
    mut q_cursor: Query<(Entity, &mut Transform), With<Cursor>>,
) {
    for (workspace, first_person) in q_workspace.iter() {
        if first_person.0 {
            for camera_e in q_camera.iter() {
                for (cursor_e, mut transform) in q_cursor.iter_mut() {
                    transform.translation.x = 0.;
                    transform.translation.y = 0.;
                    commands.insert_one(cursor_e, Parent(camera_e));
                }
            }
        } else {
            for (cursor_e, mut transform) in q_cursor.iter_mut() {
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
        (
            &Handle<ColorMaterial>,
            Option<&Hovered>,
            Option<&Selected>,
            Option<&Dragged>,
        ),
        With<Hoverable>,
    >,
) {
    let mut first = true;

    for (material, hovered, selected, dragged) in q_hoverable.iter() {
        let (red, green, blue, alpha) = if dragged.is_some() {
            (1.0, 0.0, 0.0, 1.0)
        } else if first && hovered.is_some() {
            first = false;
            (0.0, 1.0, 0.0, 1.0)
        } else if selected.is_some() {
            (0.0, 0.0, 1.0, 1.0)
        } else if hovered.is_some() {
            (1.0, 1.0, 1.0, 0.5)
        } else {
            (1.0, 1.0, 1.0, 1.0)
        };

        materials.get_mut(material).unwrap().color.set_r(red);
        materials.get_mut(material).unwrap().color.set_g(green);
        materials.get_mut(material).unwrap().color.set_b(blue);
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
    mut q_camera: Query<&mut Transform, With<Camera>>,
    q_workspace: Query<(&Workspace, &FirstPerson)>,
) {
    for (workspace, first_person) in q_workspace.iter() {
        if !first_person.0 {
            return;
        }

        for mut transform in q_camera.iter_mut() {
            transform.translation.x += workspace.cursor_delta.x;
            transform.translation.y -= workspace.cursor_delta.y;
        }
    }
}

fn cursor_visibility(
    mut windows: ResMut<Windows>,
    q_first_person: Query<&FirstPerson, Changed<FirstPerson>>,
) {
    for first_person in q_first_person.iter() {
        let window = windows.get_primary_mut().unwrap();
        window.set_cursor_visibility(!first_person.0);

        let window_size = Vec2::new(window.width(), window.height());
        if first_person.0 {
            window.set_cursor_position(window_size / 2.0);
        }
    }
}

fn crosshair_visibility(
    q_workspace: Query<&FirstPerson>,
    mut query: Query<&mut Visible, With<Crosshair>>,
) {
    for first_person in q_workspace.iter() {
        for mut visible in query.iter_mut() {
            visible.is_visible = first_person.0;
        }
    }
}

fn first_person(
    input: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    e_window_focused: Res<Events<WindowFocused>>,
    mut state_global: ResMut<StateGlobal>,
    mut q_first_person: Query<&mut FirstPerson>,
) {
    for mut first_person in q_first_person.iter_mut() {
        if input.just_pressed(KeyCode::Tab) {
            first_person.0 = !first_person.0;
        }
        if input.just_pressed(KeyCode::Escape) {
            first_person.0 = false;
        }

        if first_person.0 {
            let window = windows.get_primary_mut().unwrap();
            let window_size = Vec2::new(window.width(), window.height());
            window.set_cursor_position(window_size / 2.0);
        }

        let event_window_focused = state_global.er_window_focused.latest(&e_window_focused);
        if let Some(event_window_focused) = event_window_focused {
            if !event_window_focused.focused {
                first_person.0 = false;
            }
        }
    }
}

fn deselect(
    input: Res<Input<KeyCode>>,
    commands: &mut Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if input.just_pressed(KeyCode::A) {
        for entity in q_selected.iter() {
            commands.remove_one::<Selected>(entity);
        }
    }
}

fn select_single(
    i_mouse_button: Res<Input<MouseButton>>,
    commands: &mut Commands,
    q_hovered: Query<Entity, (With<Hovered>, Without<Selected>)>,
    q_not_hovered: Query<Entity, (Without<Hovered>, With<Selected>)>,
) {
    if !i_mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    for entity in q_not_hovered.iter() {
        commands.remove_one::<Selected>(entity);
    }

    for entity in q_hovered.iter() {
        commands.insert_one(entity, Selected);
    }
}

fn none_setup(commands: &mut Commands, q_hovered: Query<Entity, With<Hovered>>) {
    for entity in q_hovered.iter() {
        commands.remove_one::<Hovered>(entity);
    }
}

fn grab_setup(commands: &mut Commands, q_selected: Query<Entity, With<Selected>>) {
    for entity in q_selected.iter() {
        commands.insert_one(entity, Dragged);
    }
}

fn grab(mut mode: ResMut<State<ModeState>>, i_mouse_button: Res<Input<MouseButton>>) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        mode.overwrite_next(ModeState::None).unwrap();
    }
}

fn grab_cleanup(commands: &mut Commands, q_dragged: Query<Entity, With<Dragged>>) {
    for entity in q_dragged.iter() {
        commands.remove_one::<Dragged>(entity);
        commands.insert_one(entity, Dropped);
    }
}
