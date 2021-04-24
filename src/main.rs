#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod add_tool;
pub mod box_select;
pub mod camera;
pub mod mouse_interaction;
pub mod processing;
pub mod scan_code_input;
pub mod workspace_drag_drop;
pub mod workspace;
pub mod material;
pub mod sync_graph;

use bevy::{
    app::AppExit, audio::AudioPlugin, prelude::*, window::WindowFocused,
};
use kanter_core::{
    dag::TextureProcessor,
    node::Side,
    node_graph::{NodeId, SlotId},
};
use add_tool::*;
use box_select::*;
use camera::*;
use mouse_interaction::*;
use processing::*;
use scan_code_input::*;
use workspace_drag_drop::*;
use workspace::*;
use material::*;
use sync_graph::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GrabToolType {
    Add,
    Node,
    Slot,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ToolState {
    None,
    Add,
    BoxSelect,
    Delete,
    Export,
    Grab(GrabToolType),
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

struct Crosshair;
struct Instructions;
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

pub(crate) const CAMERA_DISTANCE: f32 = 10.;
pub(crate) const THUMBNAIL_SIZE: f32 = 128.;

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
        app.add_state(ToolState::None)
            .add_state(FirstPersonState::Off)
            .add_plugin(ScanCodeInputPlugin)
            .add_plugin(AddToolPlugin)
            .add_plugin(WorkspaceDragDropPlugin)
            .add_plugin(ProcessingPlugin)
            .add_plugin(MouseInteractionPlugin)
            .add_plugin(BoxSelectPlugin)
            .add_plugin(CameraPlugin)
            .add_plugin(WorkspacePlugin)
            .add_plugin(MaterialPlugin)
            .add_plugin(SyncGraphPlugin)
            .add_startup_system(setup.system())
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
                        hoverable
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None))
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            )
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(deselect.system())
                    .with_system(update_instructions.system()),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new().with_system(quit_hotkey.system()),
            )
            ;
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
                ToolState::Grab(gtt) => {
                    if *gtt == GrabToolType::Node || *gtt == GrabToolType::Add {
                        "LMB: Confirm".to_string()
                    } else {
                        return;
                    }
                }
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
                ScanCode::KeyG => Some(tool_state.set(ToolState::Grab(GrabToolType::Node))),
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

fn stretch_between(sprite: &mut Sprite, transform: &mut Transform, start: Vec2, end: Vec2) {
    let midpoint = start - (start - end) / 2.;
    let distance = start.distance(end);
    let rotation = Vec2::X.angle_between(start - end);

    transform.translation = midpoint.extend(0.0);
    transform.rotation = Quat::from_rotation_z(rotation);
    sprite.size = Vec2::new(distance, 5.);
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
