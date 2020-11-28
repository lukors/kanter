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
            .add_system(toggle_cursor.system())
            .add_system(drag.system())
            .add_system(alpha.system())
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
struct Workspace {}

#[derive(Default)]
struct FirstPerson {
    on: bool,
}

#[derive(Default)]
struct Size {
    xy: Vec2,
}

struct Draggable;
struct Dragged;

struct Hoverable;
struct Hovered;

fn hoverable(
    commands: &mut Commands,
    mut state: ResMut<State>,
    e_cursor_moved: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    q_hoverable: Query<(Entity, &Hoverable, &Transform, &Size)>,
    q_camera: Query<(&Camera, &Transform)>,
) {
    let event_cursor = state.er_cursor_moved_hover.latest(&e_cursor_moved);

    if let Some(event_cursor) = event_cursor {
        let (_cam, cam_transform) = q_camera.iter().last().unwrap();
        let window = windows.get_primary().unwrap();

        let pos_wld = cursor_to_world(window, cam_transform, event_cursor.position);

        for (entity, _hoverable, transform, size) in q_hoverable.iter() {
            let half_width = size.xy.x / 2.0;
            let half_height = size.xy.y / 2.0;

            if transform.translation.x - half_width < pos_wld.x
                && transform.translation.x + half_width > pos_wld.x
                && transform.translation.y - half_height < pos_wld.y
                && transform.translation.y + half_height > pos_wld.y
            {
                commands.insert_one(entity, Hovered);
            } else {
                commands.remove_one::<Hovered>(entity);
            }
        }
    }
}

fn alpha(
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_hovered: Query<(&Hovered, &Handle<ColorMaterial>)>,
    q_hoverable: Query<(&Hoverable, &Handle<ColorMaterial>)>,
) {
    for (_hoverable, material) in q_hoverable.iter() {
        materials.get_mut(material).unwrap().color.set_a(1.0);
    }
    for (_hovered, material) in q_hovered.iter() {
        materials.get_mut(material).unwrap().color.set_a(0.5);
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
    mut q_pressed: Query<(Entity, &Draggable, &Hovered)>,
    mut q_released: Query<(Entity, &Dragged)>,
) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        if let Some((entity, _draggable, _hovered)) = q_pressed.iter_mut().next() {
            commands.insert_one(entity, Dragged);
        }
    } else if i_mouse_button.just_released(MouseButton::Left) {
        for (entity, _dragged) in q_released.iter_mut() {
            commands.remove_one::<Dragged>(entity);
        }
    }
}

fn drag(
    windows: Res<Windows>,
    mut state: ResMut<State>,
    e_cursor_moved: Res<Events<CursorMoved>>,
    mut q_dragged: Query<(&Dragged, &mut Transform)>,
    q_camera: Query<(&Camera, &Transform)>,
) {
    for (_dragged, mut transform) in q_dragged.iter_mut() {
        let event_cursor = state.er_cursor_moved_drag.latest(&e_cursor_moved);

        if let Some(event_cursor) = event_cursor {
            let window = windows.get_primary().unwrap();
            let (_cam, cam_transform) = q_camera.iter().last().unwrap();
            let cursor_world = cursor_to_world(&window, &cam_transform, event_cursor.position);

            transform.translation.x = cursor_world.x;
            transform.translation.y = cursor_world.y;
        }
    }
}

struct Crosshair;

#[derive(Default)]
struct State {
    er_mouse_motion_fpcam: EventReader<MouseMotion>,
    er_cursor_moved_hover: EventReader<CursorMoved>,
    er_cursor_moved_drag: EventReader<CursorMoved>,
}

fn camera(
    mut state: ResMut<State>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    first_person: Res<FirstPerson>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    if !first_person.on {
        return;
    }

    let mut delta: Vec2 = Vec2::zero();
    for event in state.er_mouse_motion_fpcam.iter(&mouse_motion_events) {
        delta += event.delta;
    }
    if delta == Vec2::zero() {
        return;
    }

    for (_camera, mut transform) in query.iter_mut() {
        if !first_person.on {
            continue;
        }

        transform.translation.x += delta.x;
        transform.translation.y -= delta.y;
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
