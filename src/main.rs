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
        .add_resource(FirstPerson::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(KanterPlugin)
        .run();
}

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system(workspace.system())
            .add_system(toggle_cursor.system())
            .add_system(drag.system())
            .add_system(material.system())
            .add_system(hoverable.system())
            .add_system(draggable.system())
            .add_system(cursor_visibility.system())
            .add_system(crosshair_visibility.system())
            .add_system(camera.system());
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
        .spawn(SpriteBundle {
            material: materials.add(test_image.clone().into()),
            ..Default::default()
        })
        .with(Hoverable)
        .with(Draggable)
        .with(Size {
            xy: Vec2::new(256., 256.),
        })
        .spawn(SpriteBundle {
            material: materials.add(test_image.clone().into()),
            ..Default::default()
        })
        .with(Hoverable)
        .with(Draggable)
        .with(Size {
            xy: Vec2::new(256., 256.),
        })
        .spawn(SpriteBundle {
            material: materials.add(test_image.clone().into()),
            ..Default::default()
        })
        .with(Hoverable)
        .with(Draggable)
        .with(Size {
            xy: Vec2::new(256., 256.),
        })
        .spawn(SpriteBundle {
            material: materials.add(test_image.into()),
            ..Default::default()
        })
        .with(Hoverable)
        .with(Draggable)
        .with(Size {
            xy: Vec2::new(256., 256.),
        });
}

#[derive(Default)]
struct Workspace {
    cursor_screen: Vec2,
    cursor_world: Vec2,
    cursor_delta: Vec2,
    cursor_moved: bool,
}

#[derive(Default)]
struct FirstPerson {
    on: bool,
}

#[derive(Default)]
struct Size {
    xy: Vec2,
}

struct Draggable;
#[derive(Default)]
struct Dragged {
    anchor: Vec2,
}

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
    q_pressed: Query<(Entity, &Transform), (With<Hovered>, With<Draggable>)>,
    q_released: Query<Entity, With<Dragged>>,
    q_workspace: Query<&Workspace>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    if i_mouse_button.just_pressed(MouseButton::Left) {
        if let Some((entity, transform)) = q_pressed.iter().next() {
            let translation = Vec2::new(transform.translation.x, transform.translation.y);
            let anchor = translation - workspace.cursor_world;

            commands.insert_one(entity, Dragged { anchor });
        }
    } else if i_mouse_button.just_released(MouseButton::Left) {
        for entity in q_released.iter() {
            commands.remove_one::<Dragged>(entity);
        }
    }
}

fn drag(mut q_dragged: Query<(&Dragged, &mut Transform)>, q_workspace: Query<&Workspace>) {
    let workspace = q_workspace.iter().next().unwrap();

    if workspace.cursor_moved {
        for (dragged, mut transform) in q_dragged.iter_mut() {
            transform.translation.x = workspace.cursor_world.x + dragged.anchor.x;
            transform.translation.y = workspace.cursor_world.y + dragged.anchor.y;
        }
    }
}

struct Crosshair;

#[derive(Default)]
struct State {
    er_mouse_motion: EventReader<MouseMotion>,
    er_cursor_moved: EventReader<CursorMoved>,
}

fn camera(
    first_person: Res<FirstPerson>,
    mut q_camera: Query<&mut Transform, With<Camera>>,
    q_workspace: Query<&Workspace>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    if !first_person.on {
        return;
    }

    for mut transform in q_camera.iter_mut() {
        if !first_person.on {
            continue;
        }

        transform.translation.x += workspace.cursor_delta.x;
        transform.translation.y -= workspace.cursor_delta.y;
    }
}

fn cursor_visibility(mut windows: ResMut<Windows>, first_person: Res<FirstPerson>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_visibility(!first_person.on);

    if first_person.on {
        window.set_cursor_position((window.width() / 2) as i32, (window.height() / 2) as i32);
    }
}

fn crosshair_visibility(first_person: Res<FirstPerson>, mut query: Query<(&Crosshair, &mut Draw)>) {
    for (_crosshair, mut draw) in query.iter_mut() {
        draw.is_visible = first_person.on;
    }
}

fn toggle_cursor(mut first_person: ResMut<FirstPerson>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Tab) {
        first_person.on = !first_person.on;
    }
}
