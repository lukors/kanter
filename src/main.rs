use bevy::{
    prelude::*,
    transform::components::Transform,
};


fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "Bevy".to_string(),
            width: 1024,
            height: 768,
            vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        // .add_system(toggle_cursor)
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
        .with(Camera)
        .spawn(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}

struct MyPosition { x: f32, y: f32 }
struct Camera;

fn update_camera(mut query: Query<(&mut Transform, &Camera)>) {
    for (mut transform, my_struct) in query.iter_mut() {
        transform.translation.x += 1.;
    }
}

fn toggle_cursor(input: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    // if input.just_pressed(KeyCode::Space) {
    //     viewport.locked = !viewport.locked;
    // }

    // if viewport.locked {
    //     let window = windows.get_primary_mut().unwrap();
    //     window.set_cursor_lock_mode(!window.cursor_locked());
    //     window.set_cursor_visibility(!window.cursor_visible());
    // }
}


