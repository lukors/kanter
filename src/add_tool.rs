use std::sync::{Arc, RwLock};

/// Adding new nodes
use crate::{
    camera::Cursor,
    drag_drop::{grab_tool_cleanup, node::grab_tool_node_setup, Draggable},
    instruction::*,
    mouse_interaction::Selected,
    shared::NodeIdComponent,
    undo::{node::AddNode, prelude::*},
    AmbiguitySet, CustomStage, GrabToolType, ToolState,
};
use anyhow::{anyhow, Result};
use bevy::prelude::*;
use kanter_core::{
    live_graph::LiveGraph,
    node::{mix::MixType, node_type::NodeType, Node},
    node_graph::NodeId,
};
use native_dialog::FileDialog;

#[derive(Copy, Clone, Debug)]
struct DragToolUndo;
impl UndoCommand for DragToolUndo {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut tool_state = world.get_resource_mut::<State<ToolState>>().unwrap();
        let _ = tool_state.overwrite_replace(ToolState::Grab(GrabToolType::Node));
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
struct SelectNew;
impl UndoCommand for SelectNew {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query_filtered::<Entity, (With<Draggable>, Added<NodeIdComponent>)>();
        for entity in query.iter(world).collect::<Vec<Entity>>() {
            world.entity_mut(entity).insert(Selected);
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

/// The sneaky variant is not saved on the undo stack. Can probably be replaced with a command that
/// removes the most recent command from the undo stack.
#[derive(Copy, Clone, Debug)]
struct SelectedToCursorSneaky;
impl UndoCommand for SelectedToCursorSneaky {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query =
            world.query_filtered::<&mut Transform, (With<Selected>, With<NodeIdComponent>)>();
        let cursor = *world
            .query_filtered::<&GlobalTransform, With<Cursor>>()
            .iter(world)
            .next()
            .unwrap();

        for mut transform in query.iter_mut(world) {
            transform.translation.x = cursor.translation.x;
            transform.translation.y = cursor.translation.y;
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

/// The sneaky variant is not saved on the undo stack. Can probably be replaced with a command that
/// removes the most recent command from the undo stack.
#[derive(Copy, Clone, Debug)]
struct DeselectSneaky;
impl UndoCommand for DeselectSneaky {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query_filtered::<Entity, With<Selected>>();

        for entity in query.iter(world).collect::<Vec<Entity>>() {
            world.entity_mut(entity).remove::<Selected>();
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

pub(crate) struct AddToolPlugin;

impl Plugin for AddToolPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(CustomStage::Setup)
                    .after(CustomStage::Input)
                    .with_system(
                        add_tool_instructions
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Add))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        add_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::Add))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_add_instructions
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Add)))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_node_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Add)))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_add_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Add)))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_cleanup
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Add)))
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("Shift A: Add node".to_string());
}

fn add_tool_instructions(mut instructions: ResMut<Instructions>) {
    instructions.insert(
        InstructId::Tool,
        "C: Combine\nI: Input\nM: Mix\nN: Height to Normal\nO: Output\nV: Value\nS: Separate\n"
            .to_string(),
    );
}

/// When you press the button for a node it creates that node for you.
fn add_update(
    commands: Commands,
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut tool_state: ResMut<State<ToolState>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    let mut done = false;

    for event in char_input_events.iter() {
        let node_type: Option<NodeType> = match event.char.to_ascii_lowercase() {
            'c' => Some(NodeType::CombineRgba),
            'i' => {
                match FileDialog::new()
                    // .set_location("~/Desktop")
                    .add_filter("PNG Image", &["png"])
                    .add_filter("JPEG Image", &["jpg", "jpeg"])
                    .show_open_single_file()
                {
                    Ok(Some(path)) => Some(NodeType::Image(path)),
                    Ok(None) => {
                        warn!("Invalid path");
                        done = true;
                        None
                    }
                    Err(e) => {
                        warn!("Error bringing up file dialog: {}", e);
                        done = true;
                        None
                    }
                }
            }
            'm' => Some(NodeType::Mix(MixType::Add)),
            'n' => Some(NodeType::HeightToNormal),
            'o' => {
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

                Some(NodeType::OutputRgba("untitled".into()))
            }
            's' => Some(NodeType::SeparateRgba),
            'v' => Some(NodeType::Value(1.0)),
            _ => None,
        };

        if let Some(node_type) = node_type {
            if let Ok(node) = create_default_node(commands, &live_graph, node_type.clone()) {
                undo_command_manager.push(Box::new(AddNode::new(node, Vec2::ZERO)));
                undo_command_manager.push(Box::new(DeselectSneaky));
                undo_command_manager.push(Box::new(SelectNew));
                undo_command_manager.push(Box::new(SelectedToCursorSneaky));
                undo_command_manager.push(Box::new(DragToolUndo));
                // Not adding an undo checkpoint because it should be added after the `Node` has
                // been placed.
                info!("Added node: {:?}", node_type);
            }
            break;
        } else if done {
            tool_state.overwrite_replace(ToolState::None).unwrap();
            break;
        }
    }
}

#[derive(Component)]
pub struct NewNode(pub NodeId);

pub fn create_default_node(
    mut commands: Commands,
    live_graph: &Arc<RwLock<LiveGraph>>,
    node_type: NodeType,
) -> Result<Node> {
    let node_id = live_graph.write().map_err(|e| anyhow!("{}", e))?.new_id();
    let output = Ok(Node::with_id(node_type, node_id)
        .resize_policy(kanter_core::node::ResizePolicy::MostPixels)
        .resize_filter(kanter_core::node::ResizeFilter::Triangle));

    commands.spawn().insert(NewNode(node_id));

    output
}

fn grab_tool_add_instructions(mut instructions: ResMut<Instructions>) {
    instructions.insert(InstructId::Tool, "LMB: Confirm\n".to_string());
}

/// Exit grab tool if mouse button is pressed.
fn grab_tool_add_update(
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
) {
    if i_mouse_button.just_pressed(MouseButton::Left) {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}
