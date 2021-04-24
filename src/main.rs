#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod add_tool;
pub mod mouse_interaction;
pub mod scan_code_input;
pub mod workspace_drag_drop;

use std::{path::Path, sync::Arc};

use bevy::{
    app::AppExit,
    audio::AudioPlugin,
    input::mouse::MouseMotion,
    prelude::*,
    render::texture::{Extent3d, TextureDimension, TextureFormat},
    window::WindowFocused,
};
use kanter_core::{
    dag::TextureProcessor,
    node::{EmbeddedNodeDataId, Node, NodeType, ResizeFilter, ResizePolicy, Side},
    node_data::Size as TPSize,
    node_graph::{NodeId, SlotId},
};
use native_dialog::FileDialog;
use rand::Rng;

use add_tool::*;
use mouse_interaction::*;
use scan_code_input::*;
use workspace_drag_drop::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ToolState {
    None,
    Add,
    BoxSelect,
    Delete,
    Export,
    GrabNode,
    GrabSlot,
    GrabAdd,
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
            title: "Kanter".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
        // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
        .add_plugins_with(DefaultPlugins, |group| group.disable::<AudioPlugin>())
        .add_plugin(KanterPlugin)
        .run();
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    False,
    Starting,
    True,
    Dropping,
}

impl Default for Drag {
    fn default() -> Self {
        Drag::False
    }
}

#[derive(Default)]
struct Workspace {
    cursor_screen: Vec2,
    cursor_world: Vec2,
    cursor_delta: Vec2,
    cursor_moved: bool,
    drag: Drag,
}

struct Crosshair;
struct Instructions;
struct WorkspaceCameraAnchor;
struct WorkspaceCamera;
struct Thumbnail;
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
// I'm saving the start and end variables for when I want to select the edges themselves.
struct Edge {
    start: Vec2,
    end: Vec2,
    output_slot: Slot,
    input_slot: Slot,
}
#[derive(Default)]
struct BoxSelect {
    start: Vec2,
    end: Vec2,
}

const DRAG_THRESHOLD: f32 = 5.;
const CAMERA_DISTANCE: f32 = 10.;
const SMALLEST_DEPTH_UNIT: f32 = f32::EPSILON * 500.;

const THUMBNAIL_SIZE: f32 = 128.;
const SLOT_SIZE: f32 = 30.;
const SLOT_MARGIN: f32 = 2.;
const SLOT_DISTANCE_X: f32 = THUMBNAIL_SIZE / 2. + SLOT_SIZE / 2. + SLOT_MARGIN;
const NODE_SIZE: f32 = THUMBNAIL_SIZE + SLOT_SIZE * 2. + SLOT_MARGIN * 2.;
const SLOT_DISTANCE_Y: f32 = 32. + SLOT_MARGIN;
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
            .insert_resource(Workspace::default())
            .add_state(ToolState::None)
            .add_state(FirstPersonState::Off)
            .add_plugin(ScanCodeInputPlugin)
            .add_plugin(AddToolPlugin)
            .add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::new().with_system(workspace.system()),
            )
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Input)
                    .with_system(hotkeys.system())
                    // .with_system(print_pressed_keys.system())
                    .with_system(focus_change.system()),
            )
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Update)
                    .after(Stage::Input)
                    .with_system(
                        delete
                            .system()
                            .with_run_criteria(State::on_update(ToolState::Delete))
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
                        // # Mouse interaction
                        grab_tool_node_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::GrabNode))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_slot_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::GrabSlot))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabNode))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabSlot))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_cleanup
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::GrabNode))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        drag_node_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabNode))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_cleanup
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::GrabSlot))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grabbed_edge_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabSlot))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        dropped_edge_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabSlot))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        spawn_grabbed_edges
                            .system()
                            .with_run_criteria(State::on_update(ToolState::GrabSlot))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        mouse_interaction
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None)),
                    )
                    .with_system(
                        hoverable
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None))
                            .in_ambiguity_set(AmbiguitySet),
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
                    )
                    .with_system(
                        mouse_pan
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
                    .with_system(dropped_update.system())
                    .with_system(update_instructions.system())
                    .with_system(
                        sync_graph
                            .system()
                            .chain(drag_node_update.system())
                            .chain(update_edges.system())
                            .chain(material.system())
                            .label("material"),
                    )
                    .with_system(
                        process
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Process))
                            .after("material"),
                    ),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new().with_system(quit_hotkey.system()),
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let crosshair_image = asset_server.load("crosshair.png");

    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(WorkspaceCamera)
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Transform::from_translation(Vec3::new(
                    0.,
                    0.,
                    -CAMERA_DISTANCE,
                )))
                .insert(GlobalTransform::default())
                .insert(WorkspaceCameraAnchor)
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
                        .insert(Transform::from_translation(Vec3::new(0., 0., 9.0)))
                        .insert(Crosshair);
                });
        });

    commands
        .spawn()
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(Cursor);

    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(TextBundle {
                    style: Style {
                        align_self: AlignSelf::FlexEnd,
                        ..Default::default()
                    },
                    text: Text::with_section(
                        START_INSTRUCT,
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Regular.ttf"),
                            font_size: 20.0,
                            color: Color::WHITE,
                        },
                        Default::default(),
                    ),
                    ..Default::default()
                })
                .insert(Instructions);
        });
}

fn workspace(
    mut er_mouse_motion: EventReader<MouseMotion>,
    mut er_cursor_moved: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut workspace: ResMut<Workspace>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_camera: Query<&Transform, With<WorkspaceCamera>>,
    mut true_cursor_world: Local<Vec2>,
) {
    let mut event_cursor_delta: Vec2 = Vec2::ZERO;
    for event_motion in er_mouse_motion.iter() {
        event_cursor_delta += event_motion.delta;
    }
    let event_cursor_screen = er_cursor_moved.iter().last();

    if let Some(event_cursor_screen) = event_cursor_screen {
        workspace.cursor_screen = event_cursor_screen.position;

        let window = windows.get_primary().unwrap();
        let cam_transform = q_camera.iter().last().unwrap();

        *true_cursor_world = cursor_to_world(window, cam_transform, event_cursor_screen.position);

        workspace.cursor_moved = true;
    } else {
        workspace.cursor_moved = false;
    }

    workspace.cursor_delta = event_cursor_delta;

    if !i_mouse_button.pressed(MouseButton::Left) || workspace.drag == Drag::True {
        workspace.cursor_world = *true_cursor_world;
    }

    if workspace.drag == Drag::Dropping {
        workspace.drag = Drag::False;
    } else if workspace.drag == Drag::Starting {
        workspace.drag = Drag::True;
    }

    if i_mouse_button.just_released(MouseButton::Left) && workspace.drag == Drag::True {
        workspace.drag = Drag::Dropping;
    }

    if i_mouse_button.pressed(MouseButton::Left)
        && true_cursor_world.distance(workspace.cursor_world) > DRAG_THRESHOLD
        && workspace.drag == Drag::False
    {
        workspace.drag = Drag::Starting;
    }
}

const START_INSTRUCT: &str = &"Shift A: Add node";

fn update_instructions(
    tool_state: Res<State<ToolState>>,
    first_person_state: Res<State<FirstPersonState>>,
    q_node: Query<&NodeId>,
    mut previous_tool_state: Local<ToolState>,
    mut previous_first_person_state: Local<FirstPersonState>,
    mut q_instructions: Query<&mut Text, With<Instructions>>,
) {
    let fp_changed = *first_person_state.current() != *previous_first_person_state;
    let tool_changed = *tool_state.current() != *previous_tool_state;

    if fp_changed || tool_changed {
        const ADD_INSTRUCT: &str = &"I: Input\nO: Output";
        let node_count = q_node.iter().len();

        let instruct_text = if *tool_state.current() == ToolState::Add {
            ADD_INSTRUCT.to_string()
        } else if node_count == 0 {
            START_INSTRUCT.to_string()
        } else {
            let none_instruct =
                "F12: Process graph\nShift Alt S: Save selected as\n\nG: Grab\nX: Delete\n";

            let tool = match tool_state.current() {
                ToolState::None => format!("{}\n{}", START_INSTRUCT, none_instruct),
                ToolState::Add => ADD_INSTRUCT.to_string(),
                ToolState::GrabNode => "LMB: Confirm".to_string(),
                _ => return,
            };

            let fp = {
                if *tool_state.current() == ToolState::None {
                    let state = match first_person_state.current() {
                        FirstPersonState::On => "On",
                        FirstPersonState::Off => "Off",
                    };

                    format!("`: First person ({})\n", state)
                } else {
                    String::new()
                }
            };

            format!("{}{}", fp, tool)
        };

        if let Ok(mut text) = q_instructions.single_mut() {
            text.sections[0].value = instruct_text;
        }
    }

    *previous_tool_state = tool_state.current().clone();
    *previous_first_person_state = first_person_state.current().clone();
}

fn focus_change(
    mut er_window_focused: EventReader<WindowFocused>,
    mut keyboard_input: ResMut<ScanCodeInput>,
) {
    if er_window_focused.iter().any(|event| !event.focused) {
        keyboard_input.clear();
    }
}

fn export(
    tex_pro: Res<TextureProcessor>,
    q_selected: Query<&NodeId, With<Selected>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut keyboard_input: ResMut<ScanCodeInput>,
) {
    for node_id in q_selected.iter() {
        let size: TPSize = match tex_pro.get_node_size(*node_id) {
            Some(s) => s,
            None => {
                info!("Unable to get the size of the node");
                continue;
            }
        };

        let path = match FileDialog::new()
            // .set_location("~/Desktop")
            .add_filter("PNG Image", &["png"])
            .show_save_single_file()
        {
            Ok(path) => path,
            Err(e) => {
                warn!("Unable to get export path: {:?}\n", e);
                continue;
            }
        };

        let path = match path {
            Some(path) => path,
            None => {
                warn!("Invalid export path");
                continue;
            }
        };

        let texels = match tex_pro.get_output(*node_id) {
            Ok(buf) => buf,
            Err(e) => {
                error!("Error when trying to get pixels from image: {:?}", e);
                continue;
            }
        };

        let buffer = match image::RgbaImage::from_vec(size.width, size.height, texels) {
            None => {
                error!("Output image buffer not big enough to contain texels.");
                continue;
            }
            Some(buf) => buf,
        };

        match image::save_buffer(
            &Path::new(&path),
            &buffer,
            size.width,
            size.height,
            image::ColorType::RGBA(8),
        ) {
            Ok(_) => info!("Image exported to {:?}", path),
            Err(e) => {
                error!("{}", e);
                continue;
            }
        }
    }

    keyboard_input.clear();
    tool_state.overwrite_replace(ToolState::None).unwrap();
}

fn process(
    mut tex_pro: ResMut<TextureProcessor>,
    mut tool_state: ResMut<State<ToolState>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_thumbnail: Query<(Entity, &Parent), With<Thumbnail>>,
    q_node: Query<(Entity, &NodeId)>,
) {
    info!("Processing graph...");
    tex_pro.process();

    info!("Generating thumbnails...");
    for (node_e, node_id) in q_node.iter() {
        if let Some(texture) = generate_thumbnail(
            &tex_pro,
            *node_id,
            Size::new(THUMBNAIL_SIZE as f32, THUMBNAIL_SIZE as f32),
        ) {
            let texture_handle = textures.add(texture);

            if let Some((thumbnail_e, _)) = q_thumbnail
                .iter()
                .find(|(_, parent_e)| parent_e.0 == node_e)
            {
                commands
                    .entity(thumbnail_e)
                    .insert(materials.add(texture_handle.into()));
            }
        }
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
    info!("Done");
}

fn quit_hotkey(input: Res<ScanCodeInput>, mut app_exit_events: EventWriter<AppExit>) {
    if control_pressed(&input) && input.just_pressed(ScanCode::KeyQ) {
        app_exit_events.send(AppExit);
    }
}

fn control_pressed(scan_code_input: &Res<ScanCodeInput>) -> bool {
    scan_code_input.pressed(ScanCode::ControlLeft)
        || scan_code_input.pressed(ScanCode::ControlRight)
}
fn shift_pressed(scan_code_input: &Res<ScanCodeInput>) -> bool {
    scan_code_input.pressed(ScanCode::ShiftLeft) || scan_code_input.pressed(ScanCode::ShiftRight)
}
fn alt_pressed(scan_code_input: &Res<ScanCodeInput>) -> bool {
    scan_code_input.pressed(ScanCode::AltLeft) || scan_code_input.pressed(ScanCode::AltRight)
}

#[allow(dead_code)]
fn print_pressed_keys(scan_code_input: Res<ScanCodeInput>) {
    for code in scan_code_input.get_just_pressed() {
        info!("ScanCode: {:?}", code);
    }
}

fn hotkeys(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
    sc_input: Res<ScanCodeInput>,
) {
    if sc_input.just_pressed(ScanCode::Backquote) {
        if *first_person_state.current() == FirstPersonState::Off {
            first_person_state.set(FirstPersonState::On).unwrap();
        } else {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }

    let tool_current = tool_state.current().clone();

    if tool_current == ToolState::None {
        for key_code in sc_input.get_just_pressed() {
            let new_tool = match key_code {
                ScanCode::Delete | ScanCode::KeyX => Some(tool_state.set(ToolState::Delete)),
                ScanCode::F12 => Some(tool_state.set(ToolState::Process)),
                ScanCode::KeyA => {
                    if shift_pressed(&sc_input) {
                        Some(tool_state.set(ToolState::Add))
                    } else {
                        None
                    }
                }
                ScanCode::KeyG => Some(tool_state.set(ToolState::GrabNode)),
                ScanCode::KeyS => {
                    if alt_pressed(&sc_input) && shift_pressed(&sc_input) {
                        Some(tool_state.set(ToolState::Export))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(new_tool) = new_tool {
                new_tool.unwrap();
                break;
            }
        }
    } else if cancel_just_pressed(&sc_input, &i_mouse_button) && tool_current != ToolState::None {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

fn cancel_just_pressed(
    scan_code_input: &Res<ScanCodeInput>,
    i_mouse_button: &Res<Input<MouseButton>>,
) -> bool {
    scan_code_input.just_pressed(ScanCode::Escape)
        || i_mouse_button.just_pressed(MouseButton::Right)
}

fn delete(
    mut tool_state: ResMut<State<ToolState>>,
    mut tex_pro: ResMut<TextureProcessor>,
    q_selected_nodes: Query<&NodeId, With<Selected>>,
) {
    for node_id in q_selected_nodes.iter() {
        match tex_pro.node_graph.remove_node(*node_id) {
            Ok(_) => (),
            Err(e) => warn!("Unable to remove node with id {}: {}", node_id, e),
        }
    }

    tool_state.overwrite_replace(ToolState::None).unwrap();
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

fn spawn_gui_node(
    commands: &mut Commands,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    node: &Arc<Node>,
) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
            sprite: Sprite::new(Vec2::new(NODE_SIZE, NODE_SIZE)),
            transform: Transform::from_translation(Vec3::new(
                0.,
                0.,
                rand::thread_rng().gen_range(0.0..9.0),
            )),
            ..Default::default()
        })
        .insert(Hoverable)
        .insert(Selected)
        .insert(Draggable)
        .insert(Dragged)
        .insert(node.node_id)
        .with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
                    sprite: Sprite::new(Vec2::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)),
                    transform: Transform::from_translation(Vec3::new(0., 0., SMALLEST_DEPTH_UNIT)),
                    ..Default::default()
                })
                .insert(Thumbnail);
            for i in 0..node.capacity(Side::Input) {
                parent
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
                        sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        transform: Transform::from_translation(Vec3::new(
                            -SLOT_DISTANCE_X,
                            THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE_Y * i as f32,
                            SMALLEST_DEPTH_UNIT,
                        )),
                        ..Default::default()
                    })
                    .insert(Hoverable)
                    .insert(Draggable)
                    .insert(Slot {
                        node_id: node.node_id,
                        side: Side::Input,
                        slot_id: SlotId(i as u32),
                    })
                    .id();
            }

            for i in 0..node.capacity(Side::Output) {
                if node.node_type == NodeType::OutputRgba || node.node_type == NodeType::OutputGray
                {
                    break;
                }
                parent
                    .spawn_bundle(SpriteBundle {
                        material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
                        sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        transform: Transform::from_translation(Vec3::new(
                            SLOT_DISTANCE_X,
                            THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2. - SLOT_DISTANCE_Y * i as f32,
                            SMALLEST_DEPTH_UNIT,
                        )),
                        ..Default::default()
                    })
                    .insert(Hoverable)
                    .insert(Draggable)
                    .insert(Slot {
                        node_id: node.node_id,
                        side: Side::Output,
                        slot_id: SlotId(i as u32),
                    })
                    .id();
            }
        });
}

fn sync_graph(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    q_node_id: Query<(Entity, &NodeId)>,
    q_edge: Query<Entity, With<Edge>>,
    q_slot: Query<(&Slot, &GlobalTransform)>,
    tex_pro: Res<TextureProcessor>,
) {
    if tex_pro.is_changed() {
        let tp_node_ids = tex_pro.node_graph.node_ids();
        let existing_gui_node_ids: Vec<NodeId> =
            q_node_id.iter().map(|(_, node_id)| *node_id).collect();
        let new_ids: Vec<NodeId> = tp_node_ids
            .iter()
            .filter(|tp_node_id| !existing_gui_node_ids.contains(tp_node_id))
            .copied()
            .collect();
        let removed_ids: Vec<NodeId> = existing_gui_node_ids
            .iter()
            .filter(|gui_node_id| !tp_node_ids.contains(gui_node_id))
            .copied()
            .collect();

        // Create gui nodes for any new nodes in the graph.
        for node_id in new_ids {
            let node = tex_pro.node_graph.node_with_id(node_id).unwrap();
            spawn_gui_node(&mut commands, &mut materials, &node);
        }

        // Remove the gui nodes for any nodes that don't exist in the graph.
        for (node_e, _) in q_node_id
            .iter()
            .filter(|(_, node_id)| removed_ids.contains(node_id))
        {
            commands.entity(node_e).despawn_recursive();
        }

        // Remove all edges so we can create new ones. This should be optimized to move
        // existing edges.
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

fn generate_thumbnail(
    tex_pro: &ResMut<TextureProcessor>,
    node_id: NodeId,
    size: Size,
) -> Option<Texture> {
    let mut tex_pro_thumb = TextureProcessor::new();

    let node_datas = tex_pro.get_node_data(node_id);

    let n_out = tex_pro_thumb
        .node_graph
        .add_node(Node::new(NodeType::OutputRgba))
        .unwrap();

    for (i, node_data) in node_datas.iter().take(4).enumerate() {
        if let Ok(end_id) = tex_pro_thumb
            .embed_node_data_with_id(Arc::clone(node_data), EmbeddedNodeDataId(i as u32))
        {
            let n_node_data = tex_pro_thumb
                .node_graph
                .add_node(Node::new(NodeType::NodeData(end_id)))
                .unwrap();

            let n_resize = tex_pro_thumb
                .node_graph
                .add_node(Node::new(NodeType::Resize(
                    Some(ResizePolicy::SpecificSize(TPSize::new(
                        size.width as u32,
                        size.height as u32,
                    ))),
                    Some(ResizeFilter::Nearest),
                )))
                .unwrap();

            tex_pro_thumb
                .node_graph
                .connect(n_node_data, n_resize, SlotId(0), SlotId(0))
                .unwrap();

            tex_pro_thumb
                .node_graph
                .connect(n_resize, n_out, SlotId(0), node_data.slot_id)
                .unwrap()
        }
    }

    tex_pro_thumb.process();

    if let Ok(output) = tex_pro_thumb.get_output(n_out) {
        Some(Texture::new(
            Extent3d::new(size.width as u32, size.height as u32, 1),
            TextureDimension::D2,
            output,
            TextureFormat::Rgba8Unorm,
        ))
    } else {
        None
    }
}

fn box_select_setup(mut materials: ResMut<Assets<ColorMaterial>>, mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(Color::rgba(0.0, 1.0, 0.0, 0.3).into()),
            visible: Visible {
                is_visible: true,
                is_transparent: true,
            },
            ..Default::default()
        })
        .insert(BoxSelect::default());
}

fn box_select(
    mut tool_state: ResMut<State<ToolState>>,
    workspace: Res<Workspace>,
    mut q_box_select: Query<(&mut Transform, &mut Sprite, &mut BoxSelect)>,
    q_draggable: Query<
        (Entity, &GlobalTransform, &Sprite),
        (With<Draggable>, Without<BoxSelect>, Without<Slot>),
    >,
    mut commands: Commands,
) {
    if let Ok((mut transform, mut sprite, mut box_select)) = q_box_select.single_mut() {
        if workspace.drag == Drag::Starting {
            box_select.start = workspace.cursor_world;
        }

        if workspace.drag == Drag::Dropping && *tool_state.current() != ToolState::None {
            tool_state.overwrite_replace(ToolState::None).unwrap();
            return;
        }

        box_select.end = workspace.cursor_world;

        let new_transform = Transform {
            translation: ((box_select.start + box_select.end) / 2.0).extend(CAMERA_DISTANCE),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        sprite.size = box_select.start - box_select.end;

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

fn box_select_cleanup(mut commands: Commands, q_box_select: Query<Entity, With<BoxSelect>>) {
    for q_box_select_e in q_box_select.iter() {
        commands.entity(q_box_select_e).despawn();
    }
}

fn hoverable(
    mut commands: Commands,
    workspace: Res<Workspace>,
    q_hoverable: Query<(Entity, &GlobalTransform, &Sprite), (With<Hoverable>, Without<Dragged>)>,
) {
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
    q_node: Query<
        (
            &Handle<ColorMaterial>,
            Option<&Hovered>,
            Option<&Selected>,
            Option<&Dragged>,
        ),
        With<NodeId>,
    >,
    q_slot: Query<
        (
            &Handle<ColorMaterial>,
            Option<&Hovered>,
            Option<&Selected>,
            Option<&Dragged>,
        ),
        With<Slot>,
    >,
) {
    for (material, hovered, selected, dragged) in q_node.iter() {
        if let Some(material) = materials.get_mut(material) {
            let value = if dragged.is_some() {
                0.9
            } else if selected.is_some() {
                0.75
            } else if hovered.is_some() {
                0.6
            } else {
                0.4
            };

            material.color = Color::Rgba {
                red: value,
                green: value,
                blue: value,
                alpha: 1.0,
            };
        }
    }

    for (material, hovered, selected, dragged) in q_slot.iter() {
        if let Some(material) = materials.get_mut(material) {
            let value = if dragged.is_some() {
                0.0
            } else if selected.is_some() {
                0.2
            } else if hovered.is_some() {
                0.5
            } else {
                0.3
            };

            material.color = Color::Rgba {
                red: value,
                green: value,
                blue: value,
                alpha: 1.0,
            };
        }
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

fn stretch_between(sprite: &mut Sprite, transform: &mut Transform, start: Vec2, end: Vec2) {
    let midpoint = start - (start - end) / 2.;
    let distance = start.distance(end);
    let rotation = Vec2::X.angle_between(start - end);

    transform.translation = midpoint.extend(0.0);
    transform.rotation = Quat::from_rotation_z(rotation);
    sprite.size = Vec2::new(distance, 5.);
}

fn first_person_on_update(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut er_window_focused: EventReader<WindowFocused>,
    mut windows: ResMut<Windows>,
    mut q_camera: Query<(Entity, &mut Transform), With<WorkspaceCamera>>,
    workspace: Res<Workspace>,
) {
    for (_camera_e, mut transform) in q_camera.iter_mut() {
        transform.translation.x += workspace.cursor_delta.x;
        transform.translation.y -= workspace.cursor_delta.y;
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
    workspace: Res<Workspace>,
) {
    for mut transform in q_cursor.iter_mut() {
        transform.translation.x = workspace.cursor_world.x;
        transform.translation.y = workspace.cursor_world.y;
    }
}

fn first_person_on_setup(
    mut windows: ResMut<Windows>,
    mut q_camera: Query<Entity, With<WorkspaceCameraAnchor>>,
    mut q_cursor: Query<(Entity, &mut Transform), With<Cursor>>,
    mut q_crosshair: Query<&mut Visible, With<Crosshair>>,
    mut commands: Commands,
) {
    let window = windows.get_primary_mut().unwrap();
    window.set_cursor_visibility(false);

    if let Ok(mut crosshair) = q_crosshair.single_mut() {
        crosshair.is_visible = true;
    }

    if let Ok(camera_e) = q_camera.single_mut() {
        if let Ok((cursor_e, mut transform)) = q_cursor.single_mut() {
            transform.translation.x = 0.;
            transform.translation.y = 0.;
            commands.entity(camera_e).push_children(&[cursor_e]);
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

/// This function should be turned into a tool and the hotkey should live in the hotkey system.
fn deselect(
    input: Res<ScanCodeInput>,
    mut commands: Commands,
    q_selected: Query<Entity, With<Selected>>,
) {
    if input.just_pressed(ScanCode::KeyA) {
        for entity in q_selected.iter() {
            commands.entity(entity).remove::<Selected>();
        }
    }
}
