#![allow(clippy::type_complexity)]

use bevy::{
    app::AppExit,
    input::mouse::MouseMotion,
    prelude::*,
    render::{
        camera::Camera,
        texture::{Extent3d, TextureDimension, TextureFormat},
    },
    window::WindowFocused,
};
use kanter_core::{
    dag::TextureProcessor,
    node::{Node, NodeType, ResizePolicy},
    node_data::Size,
    node_graph::SlotId,
};
use native_dialog::FileDialog;

const MODE: &str = "mode";
const FIRST_PERSON: &str = "first_person";

fn main() {
    App::build()
        .init_resource::<StateGlobal>()
        .insert_resource(WindowDescriptor {
            title: "Bevy".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: true,
            ..Default::default()
        })
        .insert_resource(State::new(ModeState::None))
        .insert_resource(State::new(FirstPersonState::Off))
        .add_stage_before(stage::UPDATE, MODE, StateStage::<ModeState>::default())
        .add_stage_after(
            MODE,
            FIRST_PERSON,
            StateStage::<FirstPersonState>::default(),
        )
        .add_plugins(DefaultPlugins)
        .add_plugin(KanterPlugin)
        .run();
}

#[derive(Clone, Copy, PartialEq)]
enum ModeState {
    None,
    Add,
    BoxSelect,
    Grab,
}

impl Default for ModeState {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Copy, PartialEq)]
enum FirstPersonState {
    Off,
    On,
}

impl Default for FirstPersonState {
    fn default() -> Self {
        Self::Off
    }
}

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_to_stage(stage::PRE_UPDATE, workspace.system())
            .add_system_to_stage(stage::PRE_UPDATE, mode.system())
            .add_system_to_stage(stage::PRE_UPDATE, first_person_input.system())
            .add_system_to_stage(stage::PRE_UPDATE, quit_hotkey.system())
            .on_state_enter(MODE, ModeState::Add, add_setup.system())
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
            .on_state_enter(
                FIRST_PERSON,
                FirstPersonState::On,
                first_person_on_setup.system(),
            )
            .on_state_update(
                FIRST_PERSON,
                FirstPersonState::On,
                first_person_on_update.system(),
            )
            .on_state_exit(
                FIRST_PERSON,
                FirstPersonState::On,
                first_person_on_cleanup.system(),
            )
            .on_state_update(
                FIRST_PERSON,
                FirstPersonState::Off,
                first_person_off_update.system(),
            )
            .add_system_to_stage(stage::UPDATE, deselect.system())
            .add_system_to_stage(stage::POST_UPDATE, drag.system())
            .add_system_to_stage(stage::POST_UPDATE, drop.system())
            .add_system_to_stage(stage::POST_UPDATE, material.system());
    }
}

const NODE_SIZE: f32 = 128.;
const NODE_SIZE_VEC: Vec2 = Vec2 {
    x: NODE_SIZE,
    y: NODE_SIZE,
};

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let test_image = asset_server.load("image_2.png");
    let crosshair_image = asset_server.load("crosshair.png");

    commands
        .spawn((Workspace::default(), ()))
        .spawn(Camera2dBundle::default())
        .with_children(|parent| {
            parent
                .spawn(SpriteBundle {
                    material: materials.add(crosshair_image.into()),
                    visible: Visible {
                        is_visible: false,
                        is_transparent: true,
                    },
                    ..Default::default()
                })
                .with(Crosshair);
        })
        .spawn((Transform::default(), GlobalTransform::default(), Cursor));

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

#[derive(Default)]
struct Workspace {
    cursor_screen: Vec2,
    cursor_world: Vec2,
    cursor_delta: Vec2,
    cursor_moved: bool,
}

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
    windows: Res<Windows>,
    mut q_workspace: Query<&mut Workspace>,
    q_camera: Query<&Transform, With<Camera>>,
) {
    let mut event_cursor_delta: Vec2 = Vec2::zero();
    for event_motion in state_global.er_mouse_motion.iter() {
        event_cursor_delta += event_motion.delta;
    }
    let event_cursor_screen = state_global.er_cursor_moved.iter().last();

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

fn quit_hotkey(input: Res<Input<KeyCode>>, mut app_exit_events: EventWriter<AppExit>) {
    if (input.pressed(KeyCode::RControl) || input.pressed(KeyCode::LControl))
        && input.just_pressed(KeyCode::Q)
    {
        app_exit_events.send(AppExit);
    }
}

fn mode(mut mode: ResMut<State<ModeState>>, input: Res<Input<KeyCode>>) {
    let mode_current = *mode.current();

    if input.just_pressed(KeyCode::Escape) && mode_current != ModeState::None {
        mode.set_next(ModeState::None).unwrap();
    }

    // Gate to avoid cancelling a running mode.
    if mode_current != ModeState::None {
        return;
    }

    if input.just_pressed(KeyCode::B) {
        mode.set_next(ModeState::BoxSelect).unwrap();
    }

    if input.just_pressed(KeyCode::G) {
        mode.set_next(ModeState::Grab).unwrap();
    }

    if (input.pressed(KeyCode::LShift) || input.pressed(KeyCode::RShift))
        && input.just_pressed(KeyCode::A)
    {
        mode.set_next(ModeState::Add).unwrap();
    }
}

fn first_person_input(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut state_global: ResMut<StateGlobal>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        if *first_person_state.current() == FirstPersonState::Off {
            first_person_state.set_next(FirstPersonState::On).unwrap();
        } else {
            first_person_state.set_next(FirstPersonState::Off).unwrap();
        }
    }

    let event_window_focused = state_global.er_window_focused.iter().last();
    if let Some(event_window_focused) = event_window_focused {
        if !event_window_focused.focused && *first_person_state.current() != FirstPersonState::Off {
            first_person_state.set_next(FirstPersonState::Off).unwrap();
        }
    }
}

fn box_select_setup(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
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

fn add_setup(
    mut commands: Commands,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let path = FileDialog::new()
        // .set_location("~/Desktop")
        .add_filter("PNG Image", &["png"])
        .add_filter("JPEG Image", &["jpg", "jpeg"])
        .show_open_single_file()
        .unwrap();

    let path = match path {
        Some(path) => path,
        None => return,
    };

    let mut tex_pro = TextureProcessor::new();

    let n_in = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Image(
            path.into_os_string().into_string().unwrap(),
        )))
        .unwrap();
    let n_resize_1 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                NODE_SIZE as u32,
                NODE_SIZE as u32,
            ))),
            None,
        )))
        .unwrap();
    let n_resize_2 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                NODE_SIZE as u32,
                NODE_SIZE as u32,
            ))),
            None,
        )))
        .unwrap();
    let n_resize_3 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                NODE_SIZE as u32,
                NODE_SIZE as u32,
            ))),
            None,
        )))
        .unwrap();
    let n_resize_4 = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::Resize(
            Some(ResizePolicy::SpecificSize(Size::new(
                NODE_SIZE as u32,
                NODE_SIZE as u32,
            ))),
            None,
        )))
        .unwrap();
    let n_out = tex_pro
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    tex_pro
        .node_graph
        .connect(n_in, n_resize_1, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_in, n_resize_2, SlotId(1), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_in, n_resize_3, SlotId(2), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_in, n_resize_4, SlotId(3), SlotId(0))
        .unwrap();

    tex_pro
        .node_graph
        .connect(n_resize_1, n_out, SlotId(0), SlotId(0))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_resize_2, n_out, SlotId(0), SlotId(1))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_resize_3, n_out, SlotId(0), SlotId(2))
        .unwrap();
    tex_pro
        .node_graph
        .connect(n_resize_4, n_out, SlotId(0), SlotId(3))
        .unwrap();

    tex_pro.process();

    let texture = Texture::new(
        Extent3d::new(NODE_SIZE as u32, NODE_SIZE as u32, 1),
        TextureDimension::D2,
        tex_pro.get_output(n_out).unwrap(),
        TextureFormat::Rgba8Unorm,
    );
    let image = textures.add(texture);
    commands
        .spawn(SpriteBundle {
            material: materials.add(image.into()),
            sprite: Sprite::new(NODE_SIZE_VEC),
            ..Default::default()
        })
        .with(Hoverable)
        .with(Draggable);
}

fn box_select(
    i_mouse_button: Res<Input<MouseButton>>,
    mut mode: ResMut<State<ModeState>>,
    q_workspace: Query<&Workspace>,
    mut q_box_select_cursor: Query<&mut Transform, With<BoxSelectCursor>>,
    mut q_box_select: Query<(&mut Transform, &mut Visible, &Sprite, &mut BoxSelect)>,
    q_draggable: Query<(Entity, &Transform, &Sprite), With<Draggable>>,
    mut commands: Commands,
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
                    commands.insert(entity, Selected);
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
    mut commands: Commands,
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

fn hoverable(
    mut commands: Commands,
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
                commands.insert(entity, Hovered);
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
    mut commands: Commands,
    i_mouse_button: Res<Input<MouseButton>>,
    q_pressed: Query<Entity, (With<Hovered>, With<Draggable>)>,
    q_released: Query<Entity, With<Dragged>>,
) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        if let Some(entity) = q_pressed.iter().next() {
            commands.insert(entity, Dragged);
        }
    } else if i_mouse_button.just_released(MouseButton::Left) {
        for entity in q_released.iter() {
            commands.remove_one::<Dragged>(entity);

            commands.insert(entity, Dropped);
        }
    }
}

fn drag(
    mut commands: Commands,
    mut q_dragged: Query<(Entity, &mut Transform, &GlobalTransform), Added<Dragged>>,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
) {
    if let Some((cursor_e, cursor_transform)) = q_cursor.iter().next() {
        for (entity, mut transform, global_transform) in q_dragged.iter_mut() {
            let global_pos = global_transform.translation - cursor_transform.translation;

            commands.insert(entity, Parent(cursor_e));

            transform.translation.x = global_pos.x;
            transform.translation.y = global_pos.y;
        }
    }
}

fn drop(
    mut commands: Commands,
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

fn first_person_on_update(
    mut windows: ResMut<Windows>,
    mut q_camera: Query<(Entity, &mut Transform), With<Camera>>,
    q_workspace: Query<&Workspace>,
) {
    for workspace in q_workspace.iter() {
        for (_camera_e, mut transform) in q_camera.iter_mut() {
            transform.translation.x += workspace.cursor_delta.x;
            transform.translation.y -= workspace.cursor_delta.y;
        }
    }

    let window = windows.get_primary_mut().unwrap();
    let window_size = Vec2::new(window.width(), window.height());
    window.set_cursor_position(window_size / 2.0);
}

fn first_person_off_update(
    mut q_cursor: Query<&mut Transform, With<Cursor>>,
    q_workspace: Query<&Workspace>,
) {
    for workspace in q_workspace.iter() {
        for mut transform in q_cursor.iter_mut() {
            transform.translation.x = workspace.cursor_world.x;
            transform.translation.y = workspace.cursor_world.y;
        }
    }
}

fn first_person_on_setup(
    mut windows: ResMut<Windows>,
    mut q_camera: Query<Entity, With<Camera>>,
    mut q_cursor: Query<(Entity, &mut Transform), With<Cursor>>,
    mut q_crosshair: Query<&mut Visible, With<Crosshair>>,
    mut commands: Commands,
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_visibility(false);

    for mut crosshair in q_crosshair.iter_mut() {
        crosshair.is_visible = true;
    }

    for (cursor_e, _transform) in q_cursor.iter_mut() {
        commands.remove_one::<Parent>(cursor_e);
    }

    for camera_e in q_camera.iter_mut() {
        for (cursor_e, mut transform) in q_cursor.iter_mut() {
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            commands.insert(cursor_e, Parent(camera_e));
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
        commands.remove_one::<Parent>(cursor_e);
    }
}

fn deselect(
    input: Res<Input<KeyCode>>,
    mut commands: Commands,
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
    mut commands: Commands,
    q_hovered: Query<Entity, (With<Hovered>, Without<Selected>)>,
    q_selected: Query<Entity, With<Selected>>,
) {
    if !i_mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    for entity in q_selected.iter() {
        commands.remove_one::<Selected>(entity);
    }

    if let Some(entity) = q_hovered.iter().next() {
        commands.insert(entity, Selected);
    }
}

fn none_setup(mut commands: Commands, q_hovered: Query<Entity, With<Hovered>>) {
    for entity in q_hovered.iter() {
        commands.remove_one::<Hovered>(entity);
    }
}

fn grab_setup(
    mut mode: ResMut<State<ModeState>>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if q_selected.iter().count() == 0 {
        mode.overwrite_next(ModeState::None).unwrap();
    }

    for entity in q_selected.iter() {
        commands.insert(entity, Dragged);
    }
}

fn grab(mut mode: ResMut<State<ModeState>>, i_mouse_button: Res<Input<MouseButton>>) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        mode.overwrite_next(ModeState::None).unwrap();
    }
}

fn grab_cleanup(mut commands: Commands, q_dragged: Query<Entity, With<Dragged>>) {
    for entity in q_dragged.iter() {
        commands.remove_one::<Dragged>(entity);
        commands.insert(entity, Dropped);
    }
}
