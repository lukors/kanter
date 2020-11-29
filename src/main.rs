#![allow(clippy::type_complexity)]

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::{camera::Camera, draw::Draw},
};

fn main() {
    App::build()
        .init_resource::<State>()
        .add_resource(WindowDescriptor {
            title: "Bevy".to_string(),
            width: 1024,
            height: 768,
            vsync: true,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(KanterPlugin)
        .run();
}

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_to_stage(stage::PRE_UPDATE, workspace.system())
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

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let test_image = asset_server.load("image_2.png");
    let crosshair_image = asset_server.load("crosshair.png");

    commands
        .spawn(())
        .with(Workspace::default())
        .spawn(Camera2dBundle::default())
        .with_children(|parent| {
            parent
                .spawn(SpriteBundle {
                    material: materials.add(crosshair_image.into()),
                    ..Default::default()
                })
                .with(Crosshair);
        })
        .spawn((Transform::default(), GlobalTransform::default(), Cursor));

    for _ in 0..4 {
        commands
            .spawn(SpriteBundle {
                material: materials.add(test_image.clone().into()),
                ..Default::default()
            })
            .with(Hoverable)
            .with(Draggable)
            .with(Size {
                xy: Vec2::new(256., 256.),
            });
    }
}

// TODO: Add a camera entity component to workspace so its more reliable to get to.
// TODO: Parent everything to the workspace it belongs to, so everything automatically is removed
//       when the workspace is.
// TODO: Box select
// TODO: Add an "active workspace" global resource.
// TODO: Stop grabbing the mouse if the window is not active. No good way to do this yet.
// TODO: Add click and drag panning

#[derive(Default)]
struct Workspace {
    cursor_screen: Vec2,
    cursor_world: Vec2,
    cursor_delta: Vec2,
    cursor_moved: bool,
    first_person: bool,
}

#[derive(Default)]
struct Size {
    xy: Vec2,
}

struct Cursor;

struct Draggable;
#[derive(Default)]
struct Dragged;
struct Dropped;

struct Hoverable;
struct Hovered;

fn workspace(
    mut state: ResMut<State>,
    e_cursor_moved: Res<Events<CursorMoved>>,
    e_mouse_motion: Res<Events<MouseMotion>>,
    windows: Res<Windows>,
    mut q_workspace: Query<&mut Workspace>,
    q_camera: Query<&Transform, With<Camera>>,
) {
    let mut event_cursor_delta: Vec2 = Vec2::zero();
    for event_motion in state.er_mouse_motion.iter(&e_mouse_motion) {
        event_cursor_delta += event_motion.delta;
    }
    let event_cursor_screen = state.er_cursor_moved.latest(&e_cursor_moved);

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

fn cursor_transform(
    commands: &mut Commands,
    q_workspace: Query<&Workspace>,
    q_camera: Query<Entity, With<Camera>>,
    mut q_cursor: Query<(Entity, &mut Transform), With<Cursor>>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    if workspace.first_person {
        let camera_e = q_camera.iter().next().unwrap();

        for (entity, mut transform) in q_cursor.iter_mut() {
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            commands.insert_one(entity, Parent(camera_e));
        }
    } else {
        for (entity, mut transform) in q_cursor.iter_mut() {
            transform.translation.x = workspace.cursor_world.x;
            transform.translation.y = workspace.cursor_world.y;
            commands.remove_one::<Parent>(entity);
        }
    }
}

fn hoverable(
    commands: &mut Commands,
    q_workspace: Query<&Workspace>,
    q_hoverable: Query<(Entity, &Transform, &Size), (With<Hoverable>, Without<Dragged>)>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    if workspace.cursor_moved {
        for (entity, transform, size) in q_hoverable.iter() {
            let half_width = size.xy.x / 2.0;
            let half_height = size.xy.y / 2.0;

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
struct State {
    er_mouse_motion: EventReader<MouseMotion>,
    er_cursor_moved: EventReader<CursorMoved>,
}

fn camera(mut q_camera: Query<&mut Transform, With<Camera>>, q_workspace: Query<&Workspace>) {
    let workspace = q_workspace.iter().next().unwrap();

    if !workspace.first_person {
        return;
    }

    for mut transform in q_camera.iter_mut() {
        transform.translation.x += workspace.cursor_delta.x;
        transform.translation.y -= workspace.cursor_delta.y;
    }
}

// TODO: Make these two functions only run when first_person has changed
fn cursor_visibility(mut windows: ResMut<Windows>, q_workspace: Query<&Workspace>) {
    let workspace = q_workspace.iter().next().unwrap();

    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_visibility(!workspace.first_person);

    if workspace.first_person {
        window.set_cursor_position((window.width() / 2) as i32, (window.height() / 2) as i32);
    }
}

fn crosshair_visibility(q_workspace: Query<&Workspace>, mut query: Query<(&Crosshair, &mut Draw)>) {
    let workspace = q_workspace.iter().next().unwrap();

    for (_crosshair, mut draw) in query.iter_mut() {
        draw.is_visible = workspace.first_person;
    }
}

fn first_person(input: Res<Input<KeyCode>>, mut q_workspace: Query<&mut Workspace>) {
    let mut workspace = q_workspace.iter_mut().next().unwrap();

    if input.just_pressed(KeyCode::Tab) {
        workspace.first_person = !workspace.first_person;
    }
    if input.just_pressed(KeyCode::Escape) {
        workspace.first_person = false;
    }
}
