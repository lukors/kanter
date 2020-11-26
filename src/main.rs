use bevy::{input::mouse::MouseMotion, prelude::*, render::camera::Camera};

fn main() {
    App::build()
        .init_resource::<State>()
        .add_resource(WindowDescriptor {
            title: "Bevy".to_string(),
            width: 1024,
            height: 768,
            vsync: false,
            ..Default::default()
        })
        .add_resource(FirstPerson::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(toggle_cursor.system())
        .add_system(update_cursor_visibility.system())
        .add_system(update_camera.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("image_2.png");

    commands
        .spawn(Camera2dBundle::default())
        // .with(Camera)
        .spawn(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}

#[derive(Default)]
struct FirstPerson {
    on: bool,
}

// struct Camera;

#[derive(Default)]
struct State {
    mouse_motion_event_reader: EventReader<MouseMotion>,
}

fn update_camera(
    mut state: ResMut<State>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    first_person: Res<FirstPerson>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    let mut delta: Vec2 = Vec2::zero();
    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
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

fn update_cursor_visibility(mut windows: ResMut<Windows>, first_person: Res<FirstPerson>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_lock_mode(first_person.on);
    window.set_cursor_visibility(!first_person.on);
}

fn toggle_cursor(mut first_person: ResMut<FirstPerson>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Tab) {
        first_person.on = !first_person.on;
    }
}
