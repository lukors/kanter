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
use kanter_core::{dag::TextureProcessor, node::{Node, NodeType, ResizeFilter, ResizePolicy}, node_data::Size, node_graph::{NodeId, SlotId}};
use native_dialog::FileDialog;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ToolState {
    None,
    Add,
    BoxSelect,
    Grab,
}

impl Default for ToolState {
    fn default() -> Self {
        Self::None
    }
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Bevy".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: true,
            ..Default::default()
        })
        // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
        .add_plugins(DefaultPlugins)
        .add_plugin(KanterPlugin)
        .run();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum FirstPersonState {
    Off,
    On,
}

impl Default for FirstPersonState {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum Stage {
    Input,
    Update,
    Apply,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, AmbiguitySetLabel)]
struct AmbiguitySet;

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_non_send_resource(TextureProcessor::new())
            .add_startup_system(setup.system())
            .add_state(ToolState::None)
            .add_state(FirstPersonState::Off)
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new().with_system(workspace.system()),
            )
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Input)
                    .with_system(tool_input.system().chain(first_person_input.system())),
            )
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Update)
                    .after(Stage::Input)
                    .with_system(
                        add_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Add))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        box_select_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::BoxSelect))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        box_select
                            .system()
                            .with_run_criteria(State::on_update(ToolState::BoxSelect))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        box_select_cleanup
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::BoxSelect))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Grab))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab.system()
                            .with_run_criteria(State::on_update(ToolState::Grab))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_cleanup
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::Grab))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        select_single
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None)),
                    )
                    .with_system(
                        draggable
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None)),
                    )
                    .with_system(
                        hoverable
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None)),
                    )
                    .with_system(
                        none_cleanup
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::None)),
                    )
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
                    ),
            )
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(deselect.system())
                    .with_system(drag.system())
                    .with_system(drop.system())
                    .with_system(material.system())
                    .with_system(sync_graph.system()),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new().with_system(quit_hotkey.system()),
            );
    }
}

const NODE_SIZE: f32 = 128.;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let crosshair_image = asset_server.load("crosshair.png");

    commands.spawn().insert(Workspace::default());
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
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
                .insert(Crosshair);
        });
    commands
        .spawn()
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(Cursor);
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
    mut er_mouse_motion: EventReader<MouseMotion>,
    mut er_cursor_moved: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut q_workspace: Query<&mut Workspace>,
    q_camera: Query<&Transform, With<Camera>>,
) {
    let mut event_cursor_delta: Vec2 = Vec2::ZERO;
    for event_motion in er_mouse_motion.iter() {
        event_cursor_delta += event_motion.delta;
    }
    let event_cursor_screen = er_cursor_moved.iter().last();

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

fn tool_input(mut tool_state: ResMut<State<ToolState>>, input: Res<Input<KeyCode>>) {
    let tool_current = tool_state.current().clone();

    if tool_current != ToolState::None {
        if input.just_pressed(KeyCode::Escape) && tool_current != ToolState::None {
            tool_state.replace(ToolState::None).unwrap();
        }
    }

    // Gate to avoid cancelling a running tool_state by activating another tool.
    if tool_current != ToolState::None {
        return;
    }

    if input.just_pressed(KeyCode::B) {
        tool_state.set(ToolState::BoxSelect).unwrap();
    }

    if input.just_pressed(KeyCode::G) {
        tool_state.set(ToolState::Grab).unwrap();
    }

    if (input.pressed(KeyCode::LShift) || input.pressed(KeyCode::RShift))
        && input.just_pressed(KeyCode::A)
    {
        tool_state.set(ToolState::Add).unwrap();
    }
}

fn first_person_input(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut er_window_focused: EventReader<WindowFocused>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        if *first_person_state.current() == FirstPersonState::Off {
            first_person_state.set(FirstPersonState::On).unwrap();
        } else {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }

    let event_window_focused = er_window_focused.iter().last();
    if let Some(event_window_focused) = event_window_focused {
        if !event_window_focused.focused && *first_person_state.current() != FirstPersonState::Off {
            first_person_state.set(FirstPersonState::Off).unwrap();
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
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(workspace.cursor_world.extend(0.)),
            material: materials.add(crosshair_image.into()),
            ..Default::default()
        })
        .insert(BoxSelectCursor);
    commands
        .spawn_bundle(SpriteBundle {
            material: box_material,
            visible: Visible {
                is_visible: false,
                is_transparent: true,
            },
            ..Default::default()
        })
        .insert(BoxSelect::default());
}

fn add_setup(
    mut tex_pro: ResMut<TextureProcessor>,
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

    tex_pro.node_graph.add_node(Node::new(NodeType::Image(path.to_string_lossy().to_string()))).unwrap();

}

fn sync_graph(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_hoverable: Query<(Entity, &NodeId)>,
    tex_pro: Res<TextureProcessor>,
) {

    let node_ids = tex_pro.node_graph.node_ids();
    let existing_ids: Vec<NodeId> = q_hoverable.iter().map(|(_e, &node_id)| node_id).collect();
    let new_ids: Vec<NodeId> = node_ids.into_iter().filter(|node_id| !existing_ids.contains(node_id)).collect();
    // let new_ids: Vec<(Entity, &NodeId)> = q_hoverable.iter().filter(|(_e, &node_id)| !node_ids.contains(&node_id)).collect();

    for node_id in new_ids {
        let node_type = tex_pro.node_graph.node_with_id(node_id).expect("Tried getting a node that doesn't exist, this should be impossible.").node_type.clone();

    commands
        .spawn_bundle(SpriteBundle {
                material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            sprite: Sprite::new(Vec2::new(NODE_SIZE, NODE_SIZE)),
            ..Default::default()
        })
        .insert(Hoverable)
            .insert(Draggable)
            .insert(node_id)
            .insert(node_type);
}

fn box_select(
    i_mouse_button: Res<Input<MouseButton>>,
    mut tool_state: ResMut<State<ToolState>>,
    q_workspace: Query<&Workspace>,
    mut q_box_select_cursor: Query<&mut Transform, With<BoxSelectCursor>>,
    mut q_box_select: Query<
        (&mut Transform, &mut Visible, &Sprite, &mut BoxSelect),
        Without<BoxSelectCursor>,
    >,
    q_draggable: Query<
        (Entity, &GlobalTransform, &Sprite),
        (
            With<Draggable>,
            Without<BoxSelectCursor>,
            Without<BoxSelect>,
        ),
    >,
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
            && *tool_state.current() != ToolState::None
        {
            tool_state.replace(ToolState::None).unwrap();
            return;
        }

        if visible.is_visible {
            box_select.end = workspace.cursor_world;

            let new_transform = Transform {
                translation: ((box_select.start + box_select.end) / 2.0).extend(0.),
                rotation: Quat::IDENTITY,
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
                    commands.entity(entity).insert(Selected);
                } else {
                    commands.entity(entity).remove::<Selected>();
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
        commands.entity(box_select_cursor_e).despawn();
    }

    for q_box_select_e in q_box_select.iter() {
        commands.entity(q_box_select_e).despawn();
    }
}

fn hoverable(
    mut commands: Commands,
    q_workspace: Query<&Workspace>,
    q_hoverable: Query<(Entity, &GlobalTransform, &Sprite), (With<Hoverable>, Without<Dragged>)>,
) {
    let workspace = q_workspace.iter().next().unwrap();

    if workspace.cursor_moved {
        for (entity, global_transform, sprite) in q_hoverable.iter() {
            let half_width = sprite.size.x / 2.;
            let half_height = sprite.size.y / 2.;

            if global_transform.translation.x - half_width < workspace.cursor_world.x
                && global_transform.translation.x + half_width > workspace.cursor_world.x
                && global_transform.translation.y - half_height < workspace.cursor_world.y
                && global_transform.translation.y + half_height > workspace.cursor_world.y
            {
                commands.entity(entity).insert(Hovered);
            } else {
                commands.entity(entity).remove::<Hovered>();
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
            commands.entity(entity).insert(Dragged);
        }
    } else if i_mouse_button.just_released(MouseButton::Left) {
        for entity in q_released.iter() {
            commands.entity(entity).remove::<Dragged>();

            commands.entity(entity).insert(Dropped);
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

            commands.entity(entity).insert(Parent(cursor_e));

            transform.translation.x = global_pos.x;
            transform.translation.y = global_pos.y;
        }
    }
}

fn drop(mut commands: Commands, mut q_dropped: Query<Entity, Added<Dropped>>) {
    for entity in q_dropped.iter_mut() {
        commands.entity(entity).remove::<Parent>();
        commands.entity(entity).remove::<Dropped>();
    }
}

struct Crosshair;

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
        commands.entity(cursor_e).remove::<Parent>();
    }

    for camera_e in q_camera.iter_mut() {
        for (cursor_e, mut transform) in q_cursor.iter_mut() {
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            commands.entity(cursor_e).insert(Parent(camera_e));
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

fn deselect(
    input: Res<Input<KeyCode>>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if input.just_pressed(KeyCode::A) {
        for entity in q_selected.iter() {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

fn select_single(
    i_mouse_button: Res<Input<MouseButton>>,
    mut commands: Commands,
    q_hovered: Query<Entity, (With<Hovered>, Without<Selected>)>,
    q_selected: Query<Entity, With<Selected>>,
    q_dropped: Query<&Dropped>,
) {
    if !i_mouse_button.just_pressed(MouseButton::Left) || q_dropped.iter().count() > 0 {
        return;
    }

    for entity in q_selected.iter() {
        commands.entity(entity).remove::<Selected>();
    }

    if let Some(entity) = q_hovered.iter().next() {
        commands.entity(entity).insert(Selected);
    }
}

fn none_cleanup(mut commands: Commands, q_hovered: Query<Entity, With<Hovered>>) {
    for entity in q_hovered.iter() {
        commands.entity(entity).remove::<Hovered>();
    }
}

fn grab_setup(
    mut tool_state: ResMut<State<ToolState>>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if q_selected.iter().count() == 0 {
        tool_state.replace(ToolState::None).unwrap();
    }

    for entity in q_selected.iter() {
        commands.entity(entity).insert(Dragged);
    }
}

fn grab(mut tool_state: ResMut<State<ToolState>>, i_mouse_button: Res<Input<MouseButton>>) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        tool_state.replace(ToolState::None).unwrap();
    }
}

fn grab_cleanup(mut commands: Commands, q_dragged: Query<Entity, With<Dragged>>) {
    for entity in q_dragged.iter() {
        commands.entity(entity).remove::<Dragged>();
        commands.entity(entity).insert(Dropped);
    }
}
