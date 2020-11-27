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
        .add_startup_system(setup.system())
        .add_system(toggle_cursor.system())
        .add_system(update_hovered.system())
        .add_system(update_cursor_visibility.system())
        .add_system(update_crosshair_visibility.system())
        .add_system(update_camera.system())
        .run();
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
            material: materials.add(test_image.into()),
            ..Default::default()
        })
        .with(Hovered::default())
        .with(Size {
            xy: Vec2::new(256., 256.),
        });
}

#[derive(Default)]
struct FirstPerson {
    on: bool,
}

#[derive(Default)]
struct Size {
    xy: Vec2,
}

// #[derive(Default)]
// struct Dragged {
//     on: bool,
// }

#[derive(Default)]
struct Hovered {
    on: bool,
}

fn update_hovered(
    mut state: ResMut<State>,
    e_cursor_moved: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    e_mouse_motion: Res<Events<MouseMotion>>,
    mut q_hovered: Query<(&mut Hovered, &Transform, &Size)>,
    q_camera: Query<(&Camera, &Transform)>,
) {
    let mut cursor_pos: Option<Vec2> = None;
    for event in state.cursor_moved_event_reader.iter(&e_cursor_moved) {
        cursor_pos = Some(event.position);
    }

    if let Some(cursor_pos) = cursor_pos {
        let (_cam, cam_transform) = q_camera.iter().last().unwrap();

        // get the size of the window
        let window = windows.get_primary().unwrap();
        let size = Vec2::new(window.width() as f32, window.height() as f32);

        // the default orthographic projection is in pixels from the center;
        // just undo the translation
        let screen_pos = cursor_pos - size / 2.0;

        // apply the camera transform
        let pos_wld = cam_transform.compute_matrix() * screen_pos.extend(0.0).extend(1.0);

        // if pos_wld.
        for (mut hovered, transform, size) in q_hovered.iter_mut() {
            let half_width = size.xy.x / 2.0;
            let half_height = size.xy.y / 2.0;

            if transform.translation.x - half_width < pos_wld.x
                && transform.translation.y - half_height < pos_wld.y
                && transform.translation.x + half_width > pos_wld.x
                && transform.translation.x + half_height > pos_wld.y
            {
                println!("{:?}", pos_wld);
            }
        }
    }

    // for event in state.mouse_motion_event_reader.iter(&e_mouse_motion) {
    //     delta += event.delta;
    // }

    // for (mut hovered, transform) in q_hovered.iter_mut() {
    //     // if
    // }
}

struct Crosshair;

#[derive(Default)]
struct State {
    mouse_motion_event_reader: EventReader<MouseMotion>,
    cursor_moved_event_reader: EventReader<CursorMoved>,
}

fn update_camera(
    mut state: ResMut<State>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    first_person: Res<FirstPerson>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    if !first_person.on {
        return;
    }

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
    window.set_cursor_visibility(!first_person.on);

    if first_person.on {
        window.set_cursor_position((window.width() / 2) as i32, (window.height() / 2) as i32);
    }
}

fn update_crosshair_visibility(
    first_person: Res<FirstPerson>,
    mut query: Query<(&Crosshair, &mut Draw)>,
) {
    for (_crosshair, mut draw) in query.iter_mut() {
        draw.is_visible = first_person.on;
    }
}

fn toggle_cursor(mut first_person: ResMut<FirstPerson>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Tab) {
        first_person.on = !first_person.on;
    }
}
