#![allow(clippy::type_complexity)]

use std::path::Path;

use bevy::{
    app::AppExit, input::mouse::MouseMotion, prelude::*, render::camera::Camera,
    window::WindowFocused,
};
use kanter_core::{
    dag::TextureProcessor,
    node::{Node, NodeType, Side},
    node_graph::{NodeId, SlotId},
};
use native_dialog::FileDialog;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ToolState {
    None,
    Add,
    BoxSelect,
    Export,
    Grab,
    GrabEdge,
    Process,
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
                    .with_system(hotkeys.system())
                    .with_system(
                        add_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::Add)),
                    ),
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
                        export
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Export))
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
                        grab_edge
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabEdge))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        drop_edge
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabEdge))
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
                    .with_system(update_edges.system())
                    .with_system(material.system())
                    .with_system(sync_graph.system())
                    .with_system(
                        process
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Process)),
                    ),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new().with_system(quit_hotkey.system()),
            );
    }
}

const NODE_SIZE: f32 = 128.;
const SLOT_SIZE: f32 = 16.;
const SLOT_DISTANCE: f32 = 32.;

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

#[derive(Clone, Debug, PartialEq)]
struct Slot {
    node_id: NodeId,
    side: Side,
    slot_id: SlotId,
}

struct SourceSlot(Slot);

struct GrabbedEdge {
    start: Vec2,
    slot: Slot,
}
struct Edge {
    start: Vec2,
    end: Vec2,
    output_slot: Slot,
    input_slot: Slot,
}
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

fn export(
    tex_pro: Res<TextureProcessor>,
    q_selected: Query<&NodeId, With<Selected>>,
    mut tool_state: ResMut<State<ToolState>>,
) {
    for node_id in q_selected.iter() {
        let size = 256;

        let texels = match tex_pro.get_output(*node_id) {
            Ok(buf) => buf,
            Err(e) => {
                println!("Error when trying to get pixels from image: {:?}", e);
                continue;
            }
        };

        let buffer = match image::RgbaImage::from_vec(size, size, texels) {
            None => {
                println!("Output image buffer not big enough to contain texels.");
                continue;
            }
            Some(buf) => buf,
        };

        image::save_buffer(
            &Path::new(&"test.png"),
            &buffer,
            size,
            size,
            image::ColorType::RGBA(8),
        )
        .unwrap();
    }
    tool_state.replace(ToolState::None).unwrap();
}

fn process(mut tex_pro: ResMut<TextureProcessor>, mut tool_state: ResMut<State<ToolState>>) {
    tex_pro.process();
    tool_state.replace(ToolState::None).unwrap();
}

fn quit_hotkey(input: Res<Input<KeyCode>>, mut app_exit_events: EventWriter<AppExit>) {
    if (input.pressed(KeyCode::RControl) || input.pressed(KeyCode::LControl))
        && input.just_pressed(KeyCode::Q)
    {
        app_exit_events.send(AppExit);
    }
}

fn control_pressed(input: &Res<Input<KeyCode>>) -> bool {
    input.pressed(KeyCode::LControl) || input.pressed(KeyCode::RControl)
}
fn shift_pressed(input: &Res<Input<KeyCode>>) -> bool {
    input.pressed(KeyCode::LShift) || input.pressed(KeyCode::RShift)
}

fn hotkeys(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut tool_state: ResMut<State<ToolState>>,
    input: Res<Input<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        if *first_person_state.current() == FirstPersonState::Off {
            first_person_state.set(FirstPersonState::On).unwrap();
        } else {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }

    let tool_current = tool_state.current().clone();

    if tool_current == ToolState::None {
        for key_code in input.get_just_pressed() {
            let new_tool = match key_code {
                KeyCode::A => {
                    if shift_pressed(&input) {
                        Some(tool_state.set(ToolState::Add))
                    } else {
                        None
                    }
                }
                KeyCode::B => Some(tool_state.set(ToolState::BoxSelect)),
                KeyCode::E => {
                    if control_pressed(&input) {
                        Some(tool_state.set(ToolState::Export))
                    } else {
                        None
                    }
                }
                KeyCode::F12 => Some(tool_state.set(ToolState::Process)),
                KeyCode::G => Some(tool_state.set(ToolState::Grab)),
                _ => None,
            };

            if let Some(new_tool) = new_tool {
                new_tool.unwrap();
                break;
            }
        }
    } else {
        if input.just_pressed(KeyCode::Escape) && tool_current != ToolState::None {
            tool_state.replace(ToolState::None).unwrap();
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

fn add_setup() {
    // Not yet implemented
    // Should show instructions for what buttons to press, 'I' for input, 'O' for output.
}

fn add_update(
    input: Res<Input<KeyCode>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut tex_pro: ResMut<TextureProcessor>,
) {
    for input in input.get_pressed() {
        let node_type = match input {
            KeyCode::I => {
                let path = FileDialog::new()
                    // .set_location("~/Desktop")
                    .add_filter("PNG Image", &["png"])
                    .add_filter("JPEG Image", &["jpg", "jpeg"])
                    .show_open_single_file()
                    .unwrap();

                let path = match path {
                    Some(path) => path,
                    None => {
                        println!("Error: Invalid save file path");
                        return;
                    }
                };

                Some(NodeType::Image(path.to_string_lossy().to_string()))
            }
            KeyCode::O => {
                // let path = FileDialog::new()
                //     // .set_location("~/Desktop")
                //     .add_filter("PNG Image", &["png"])
                //     .show_save_single_file()
                //     .unwrap();

                // let path = match path {
                //     Some(path) => path,
                //     None => {
                //         println!("Error: Invalid open file path");
                //         return;
                //     }
                // };

                Some(NodeType::OutputRgba)
            }
            _ => None,
        };

        if let Some(node_type) = node_type {
            tex_pro.node_graph.add_node(Node::new(node_type)).unwrap();
            tool_state.replace(ToolState::Grab).unwrap();
        }
    }
}

fn update_edges(
    q_node: Query<&NodeId, With<Dragged>>,
    q_slot: Query<(&Slot, &GlobalTransform)>,
    mut q_edge: Query<(&mut Sprite, &mut Transform, &Edge)>,
) {
    for node_id in q_node.iter() {
        for (mut sprite, mut transform, edge) in q_edge.iter_mut().filter(|(_, _, edge)| {
            edge.input_slot.node_id == *node_id || edge.output_slot.node_id == *node_id
        }) {
            let (mut start, mut end) = (Vec2::ZERO, Vec2::ZERO);

            for (slot, slot_t) in q_slot.iter() {
                if slot.node_id == edge.output_slot.node_id
                    && slot.slot_id == edge.output_slot.slot_id
                    && slot.side == edge.output_slot.side
                {
                    start = slot_t.translation.truncate();
                } else if slot.node_id == edge.input_slot.node_id
                    && slot.slot_id == edge.input_slot.slot_id
                    && slot.side == edge.input_slot.side
                {
                    end = slot_t.translation.truncate();
                }
            }

            stretch_between(&mut sprite, &mut transform, start, end);
        }
    }
}

fn sync_graph(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_node_id: Query<&NodeId>,
    q_edge: Query<Entity, With<Edge>>,
    q_slot: Query<(&Slot, &GlobalTransform)>,
    tex_pro: Res<TextureProcessor>,
) {
    if tex_pro.is_changed() {
        let node_ids = tex_pro.node_graph.node_ids();
        let existing_ids: Vec<NodeId> = q_node_id.iter().map(|id| *id).collect();
        let new_ids: Vec<NodeId> = node_ids
            .into_iter()
            .filter(|node_id| !existing_ids.contains(node_id))
            .collect();

        for node_id in new_ids {
            let node_e = commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
                    sprite: Sprite::new(Vec2::new(NODE_SIZE, NODE_SIZE)),
                    ..Default::default()
                })
                .insert(Hoverable)
                .insert(Selected)
                .insert(Draggable)
                .insert(Dragged)
                .insert(node_id)
                .id();

            let node = tex_pro.node_graph.node_with_id(node_id).unwrap();

            for i in 0..node.capacity(Side::Input) {
                commands
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
                        sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        transform: Transform::from_translation(Vec3::new(
                            -NODE_SIZE / 2.,
                            NODE_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE * i as f32,
                            1.,
                        )),
                        ..Default::default()
                    })
                    .insert(Hoverable)
                    .insert(Draggable)
                    .insert(Slot {
                        node_id,
                        side: Side::Input,
                        slot_id: SlotId(i as u32),
                    })
                    .insert(Parent(node_e));
            }

            for i in 0..node.capacity(Side::Output) {
                commands
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
                        sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        transform: Transform::from_translation(Vec3::new(
                            NODE_SIZE / 2.,
                            NODE_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE * i as f32,
                            1.,
                        )),
                        ..Default::default()
                    })
                    .insert(Hoverable)
                    .insert(Draggable)
                    .insert(Slot {
                        node_id,
                        side: Side::Output,
                        slot_id: SlotId(i as u32),
                    })
                    .insert(Parent(node_e));
            }
        }

        for edge_e in q_edge.iter() {
            commands.entity(edge_e).despawn_recursive();
        }

        for edge in tex_pro.node_graph.edges.iter() {
            let output_slot = Slot {
                node_id: edge.output_id,
                side: Side::Output,
                slot_id: edge.output_slot,
            };
            let input_slot = Slot {
                node_id: edge.input_id,
                side: Side::Input,
                slot_id: edge.input_slot,
            };
            let mut start = Vec2::ZERO;
            let mut end = Vec2::ZERO;

            for (slot, slot_t) in q_slot.iter() {
                if slot.node_id == output_slot.node_id
                    && slot.slot_id == output_slot.slot_id
                    && slot.side == output_slot.side
                {
                    start = slot_t.translation.truncate();
                } else if slot.node_id == input_slot.node_id
                    && slot.slot_id == input_slot.slot_id
                    && slot.side == input_slot.side
                {
                    end = slot_t.translation.truncate();
                }
            }

            let mut sprite = Sprite::new(Vec2::new(5., 5.));
            let mut transform = Transform::default();

            stretch_between(&mut sprite, &mut transform, start, end);

            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                    sprite,
                    transform,
                    ..Default::default()
                })
                .insert(Edge {
                    input_slot,
                    output_slot,
                    start,
                    end,
                });
        }
    }
}

// fn add_image_thunb(
//     mut commands: Commands,
//     mut textures: ResMut<Assets<Texture>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
// ) {
//     let mut tex_pro_thumb = TextureProcessor::new();

//     let n_in = tex_pro_thumb
//         .node_graph
//         .add_node(Node::new(NodeType::Image(
//             path.into_os_string().into_string().unwrap(),
//         )))
//         .unwrap();
//     let n_resize_1 = tex_pro_thumb
//         .node_graph
//         .add_node(Node::new(NodeType::Resize(
//             Some(ResizePolicy::SpecificSize(Size::new(
//                 NODE_SIZE as u32,
//                 NODE_SIZE as u32,
//             ))),
//             Some(ResizeFilter::Nearest),
//         )))
//         .unwrap();
//     let n_resize_2 = tex_pro_thumb
//         .node_graph
//         .add_node(Node::new(NodeType::Resize(
//             Some(ResizePolicy::SpecificSize(Size::new(
//                 NODE_SIZE as u32,
//                 NODE_SIZE as u32,
//             ))),
//             Some(ResizeFilter::Nearest),
//         )))
//         .unwrap();
//     let n_resize_3 = tex_pro_thumb
//         .node_graph
//         .add_node(Node::new(NodeType::Resize(
//             Some(ResizePolicy::SpecificSize(Size::new(
//                 NODE_SIZE as u32,
//                 NODE_SIZE as u32,
//             ))),
//             Some(ResizeFilter::Nearest),
//         )))
//         .unwrap();
//     let n_resize_4 = tex_pro_thumb
//         .node_graph
//         .add_node(Node::new(NodeType::Resize(
//             Some(ResizePolicy::SpecificSize(Size::new(
//                 NODE_SIZE as u32,
//                 NODE_SIZE as u32,
//             ))),
//             Some(ResizeFilter::Nearest),
//         )))
//         .unwrap();
//     let n_out = tex_pro_thumb
//         .node_graph
//         .add_node(Node::new(NodeType::OutputRgba))
//         .unwrap();

//     tex_pro_thumb
//         .node_graph
//         .connect(n_in, n_resize_1, SlotId(0), SlotId(0))
//         .unwrap();
//     tex_pro_thumb
//         .node_graph
//         .connect(n_in, n_resize_2, SlotId(1), SlotId(0))
//         .unwrap();
//     tex_pro_thumb
//         .node_graph
//         .connect(n_in, n_resize_3, SlotId(2), SlotId(0))
//         .unwrap();
//     tex_pro_thumb
//         .node_graph
//         .connect(n_in, n_resize_4, SlotId(3), SlotId(0))
//         .unwrap();

//     tex_pro_thumb
//         .node_graph
//         .connect(n_resize_1, n_out, SlotId(0), SlotId(0))
//         .unwrap();
//     tex_pro_thumb
//         .node_graph
//         .connect(n_resize_2, n_out, SlotId(0), SlotId(1))
//         .unwrap();
//     tex_pro_thumb
//         .node_graph
//         .connect(n_resize_3, n_out, SlotId(0), SlotId(2))
//         .unwrap();
//     tex_pro_thumb
//         .node_graph
//         .connect(n_resize_4, n_out, SlotId(0), SlotId(3))
//         .unwrap();

//     tex_pro_thumb.process();

//     let texture = Texture::new(
//         Extent3d::new(NODE_SIZE as u32, NODE_SIZE as u32, 1),
//         TextureDimension::D2,
//         tex_pro_thumb.get_output(n_out).unwrap(),
//         TextureFormat::Rgba8Unorm,
//     );
//     let image = textures.add(texture);
//     commands
//         .spawn_bundle(SpriteBundle {
//             material: materials.add(image.into()),
//             sprite: Sprite::new(Vec2::new(NODE_SIZE, NODE_SIZE)),
//             ..Default::default()
//         })
//         .insert(Hoverable)
//         .insert(Draggable);
// }

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
            if box_contains_point(
                global_transform.translation.truncate(),
                sprite.size,
                workspace.cursor_world,
            ) {
                commands.entity(entity).insert(Hovered);
            } else {
                commands.entity(entity).remove::<Hovered>();
            }
        }
    }
}

fn box_contains_point(box_pos: Vec2, box_size: Vec2, point: Vec2) -> bool {
    let half_size = box_size / 2.;

    box_pos.x - half_size.x < point.x
        && box_pos.x + half_size.x > point.x
        && box_pos.y - half_size.y < point.y
        && box_pos.y + half_size.y > point.y
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

// fn grab_edges_from_slot(mut tex_pro: ResMut<TextureProcessor>, slot: Slot) {
//     let graph = &mut tex_pro.node_graph;

//     if slot.side == Side::Output {
//         graph.disconnect_slot(slot.node_id, slot.side, slot.slot_id);

//     } else {

//         // graph.edges_in_slot(node_id, side, slot_id);
//     }
// }

fn drag(
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut commands: Commands,
    qs_slot: QuerySet<(
        Query<(&GlobalTransform, &Slot), Added<Dragged>>,
        Query<(&GlobalTransform, &Slot)>,
    )>,
    mut q_dragged_node: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (Added<Dragged>, With<NodeId>, Without<Slot>),
    >,
    mut q_edge: Query<(&mut Visible, &Edge)>,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
    input: Res<Input<KeyCode>>,
) {
    let q_dragged_slot = qs_slot.q0();
    let q_slot = qs_slot.q1();

    if let Some((dragged_slot_gtransform, dragged_slot)) = q_dragged_slot.iter().next() {
        if control_pressed(&input) {
            match dragged_slot.side {
                Side::Output => {
                    for (mut edge_visible, edge) in q_edge
                        .iter_mut()
                        .filter(|(_, edge)| edge.output_slot == *dragged_slot)
                    {
                        edge_visible.is_visible = false;

                        if let Some((input_slot_gtransform, input_slot)) = q_slot.iter().find(|(_, slot)| {
                            slot.node_id == edge.input_slot.node_id
                                && slot.slot_id == edge.input_slot.slot_id
                                && slot.side == Side::Input
                        }) {
                            commands
                                .spawn_bundle(SpriteBundle {
                                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                                    sprite: Sprite::new(Vec2::new(5., 5.)),
                                    ..Default::default()
                                })
                                .insert(GrabbedEdge {
                                    start: input_slot_gtransform.translation.truncate(),
                                    slot: input_slot.clone(),
                                })
                                .insert(SourceSlot(dragged_slot.clone()));
                        }
                    }
                }
                Side::Input => {
                    if let Some((mut edge_visible, edge)) = q_edge
                        .iter_mut()
                        .find(|(_, edge)| edge.input_slot == *dragged_slot)
                    {
                        edge_visible.is_visible = false;

                        if let Some((output_slot_gtransform, output_slot)) = q_slot.iter().find(|(_, slot)| {
                            slot.node_id == edge.output_slot.node_id
                                && slot.slot_id == edge.output_slot.slot_id
                                && slot.side == Side::Output
                        }) {
                            commands
                                .spawn_bundle(SpriteBundle {
                                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                                    sprite: Sprite::new(Vec2::new(5., 5.)),
                                    ..Default::default()
                                })
                                .insert(GrabbedEdge {
                                    start: output_slot_gtransform.translation.truncate(),
                                    slot: output_slot.clone(),
                                })
                                .insert(SourceSlot(dragged_slot.clone()));
                        }
                    }
                }
            }
        } else {
            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                    sprite: Sprite::new(Vec2::new(5., 5.)),
                    ..Default::default()
                })
                .insert(GrabbedEdge {
                    start: dragged_slot_gtransform.translation.truncate(),
                    slot: dragged_slot.clone(),
                });
        }
        tool_state.replace(ToolState::GrabEdge).unwrap();

    } else {
        if let Ok((cursor_e, cursor_transform)) = q_cursor.single() {
            for (entity, mut transform, global_transform) in q_dragged_node.iter_mut() {
                let global_pos = global_transform.translation - cursor_transform.translation;

                commands.entity(entity).insert(Parent(cursor_e));

                transform.translation.x = global_pos.x;
                transform.translation.y = global_pos.y;
            }
        }
    }
}

fn drop(mut commands: Commands, mut q_dropped: Query<(Entity, Option<&Slot>), Added<Dropped>>) {
    for (entity, slot_id) in q_dropped.iter_mut() {
        if let Some(_) = slot_id {
        } else {
            commands.entity(entity).remove::<Parent>();
        }
        commands.entity(entity).remove::<Dropped>();
    }
}

fn grab_edge(
    mut q_edge: Query<(&mut Transform, &GrabbedEdge, &mut Sprite)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
) {
    if let Ok(cursor_t) = q_cursor.single() {
        for (mut edge_t, edge, mut sprite) in q_edge.iter_mut() {
            stretch_between(
                &mut sprite,
                &mut edge_t,
                edge.start,
                cursor_t.translation.truncate(),
            );
        }
    }
}

fn stretch_between(sprite: &mut Sprite, transform: &mut Transform, start: Vec2, end: Vec2) {
    let midpoint = start - (start - end) / 2.;
    let distance = start.distance(end);
    let rotation = Vec2::X.angle_between(start - end);

    transform.translation = midpoint.extend(0.0);
    transform.rotation = Quat::from_rotation_z(rotation);
    sprite.size = Vec2::new(distance, 5.);
}

fn drop_edge(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    mut tex_pro: ResMut<TextureProcessor>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_slot: Query<(&GlobalTransform, &Sprite, &Slot)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
    q_grabbed_edge: Query<(Entity, &GrabbedEdge, Option<&SourceSlot>)>,
    mut q_edge: Query<&mut Visible, With<Edge>>,
) {
    if i_mouse_button.just_released(MouseButton::Left) {
        let cursor_t = q_cursor.iter().next().unwrap();

        'outer: for (_, grabbed_edge, source_slot) in q_grabbed_edge.iter() {
            for (slot_t, slot_sprite, slot) in q_slot.iter() {
                if box_contains_point(
                    slot_t.translation.truncate(),
                    slot_sprite.size,
                    cursor_t.translation.truncate(),
                ) {
                    if tex_pro
                        .node_graph
                        .connect_arbitrary(
                            slot.node_id,
                            slot.side,
                            slot.slot_id,
                            grabbed_edge.slot.node_id,
                            grabbed_edge.slot.side,
                            grabbed_edge.slot.slot_id,
                        )
                        .is_ok()
                    {
                        if let Some(source_slot) = source_slot {
                            if source_slot.0 != *slot {
                                tex_pro.node_graph.disconnect_slot(source_slot.0.node_id, source_slot.0.side, source_slot.0.slot_id);
                            }
                        }
                        continue 'outer;
                    } else {
                        println!("Failed to connect nodes");
                        continue 'outer;
                    }
                }
            }
            tex_pro.node_graph.disconnect_slot(grabbed_edge.slot.node_id, grabbed_edge.slot.side, grabbed_edge.slot.slot_id)
        }

        for (edge_e, _, _) in q_grabbed_edge.iter() {
            commands.entity(edge_e).despawn_recursive();
        }

        for mut visible in q_edge.iter_mut() {
            visible.is_visible = true;
        }

        tool_state.replace(ToolState::None).unwrap();
    }
}

struct Crosshair;

fn first_person_on_update(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut er_window_focused: EventReader<WindowFocused>,
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

    if let Some(event_window_focused) = er_window_focused.iter().last() {
        if !event_window_focused.focused && *first_person_state.current() != FirstPersonState::Off {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }
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
