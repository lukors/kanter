use std::sync::{Arc, RwLock};

/// Adding new nodes
use crate::{
    camera::Cursor,
    drag_drop::{node::grab_node_setup, Draggable},
    instruction::*,
    mouse_interaction::select::{ReplaceSelection, Selected},
    shared::NodeIdComponent,
    sync_graph::NODE_SIZE,
    undo::{node::AddNode, prelude::*},
    AmbiguitySet, CustomStage, GrabToolType, ToolState,
};
use anyhow::{anyhow, Result};
use bevy::prelude::*;
use native_dialog::FileDialog;
use vismut_core::{
    live_graph::LiveGraph,
    node::{node_type::NodeType, Node},
};

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

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut query =
            world.query_filtered::<&NodeIdComponent, (With<Draggable>, Added<NodeIdComponent>)>();
        let new_node_ids = query.iter(world).map(|node_id| node_id.0).collect();

        undo_command_manager.push_front(Box::new(ReplaceSelection(new_node_ids)));
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

/// The sneaky variant is not saved on the undo stack. Can probably be replaced with a command that
/// removes the most recent command from the undo stack.
#[derive(Copy, Clone, Debug)]
pub(crate) struct SelectedToCursorSneaky;
impl UndoCommand for SelectedToCursorSneaky {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query =
            world.query_filtered::<(&mut Transform, &mut GlobalTransform), (With<Selected>, With<NodeIdComponent>)>();
        let cursor = *world
            .query_filtered::<&GlobalTransform, With<Cursor>>()
            .iter(world)
            .next()
            .unwrap();

        for (mut transform, mut global_transform) in query.iter_mut(world) {
            transform.translation.x = cursor.translation.x;
            transform.translation.y = cursor.translation.y;

            // I've read that I shouldn't edit the `GlobalTransform`, but it makes this case work.
            global_transform.translation.x = cursor.translation.x;
            global_transform.translation.y = cursor.translation.y;
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

/// Applies offsets when multiple nodes have been created at once, so they don't overlap.
#[derive(Copy, Clone, Debug)]
pub struct MultiImportOffset;
impl UndoCommand for MultiImportOffset {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        const NODE_OFFSET: f32 = NODE_SIZE + 12.0;

        let cursor_transform = *world
            .query_filtered::<&GlobalTransform, With<Cursor>>()
            .iter(world)
            .next()
            .unwrap();

        let mut q_new_node =
            world.query_filtered::<(&mut Transform, &mut GlobalTransform), With<Selected>>();

        for (i, (mut transform, mut global_transform)) in q_new_node.iter_mut(world).enumerate() {
            let new_translation = {
                let mut translation = transform.translation;
                translation.x = 0.0;
                translation.y = NODE_OFFSET * i as f32;
                translation
            };
            let new_global_translation = cursor_transform.translation - new_translation;

            transform.translation = new_translation;
            global_transform.translation = new_global_translation;
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("command is never placed on undo stack");
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
                        grab_node_setup
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Add)))
                            .in_ambiguity_set(AmbiguitySet),
                    )
                    .with_system(
                        grab_tool_add_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Add)))
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
        "I: Image\nS: Separate\nC: Combine\nO: Output\nV: Value\n"
            // "C: Combine\nI: Input\nM: Mix\nN: Height to Normal\nO: Output\nV: Value\nS: Separate\n"
            .to_string(),
    );
}

/// When you press the button for a node it creates that node for you.
fn add_update(
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut tool_state: ResMut<State<ToolState>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    let mut done = false;

    for event in char_input_events.iter() {
        let node_types: Vec<NodeType> = match event.char.to_ascii_lowercase() {
            'c' => vec![NodeType::CombineRgba],
            'i' => {
                let file_dialog = FileDialog::new()
                    .add_filter(
                        "Image",
                        &["bmp", "gif", "jpg", "jpeg", "png", "tga", "tiff"],
                    )
                    .show_open_multiple_file();

                if let Ok(path_bufs) = file_dialog {
                    path_bufs.into_iter().map(NodeType::Image).collect()
                } else {
                    error!("could not open file dialog");
                    done = true;
                    Vec::new()
                }
            }
            // 'm' => vec![NodeType::Mix(MixType::Add)],
            // 'n' => vec![NodeType::HeightToNormal],
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

                vec![NodeType::OutputRgba("untitled".into())]
            }
            's' => vec![NodeType::SeparateRgba],
            'v' => vec![NodeType::Value(1.0)],
            _ => Vec::new(),
        };

        if !node_types.is_empty() {
            let mut created_nodes: usize = 0;

            for node_type in node_types {
                if create_node(&mut undo_command_manager, &*live_graph, &node_type).is_ok() {
                    created_nodes += 1;
                } else {
                    warn!("failed to create node: {:?}", node_type);
                }
            }

            grab_new_nodes(&mut undo_command_manager);

            if created_nodes > 1 {
                undo_command_manager.push(Box::new(MultiImportOffset));
            }

            break;
        } else if done {
            tool_state.overwrite_replace(ToolState::None).unwrap();
            break;
        }
    }
}

pub fn create_node(
    undo_command_manager: &mut UndoCommandManager,
    live_graph: &Arc<RwLock<LiveGraph>>,
    node_type: &NodeType,
) -> Result<()> {
    let node = create_default_node(live_graph, node_type.clone())?;

    undo_command_manager.push(Box::new(AddNode::new(node, Vec2::ZERO)));

    Ok(())
}

pub fn grab_new_nodes(undo_command_manager: &mut UndoCommandManager) {
    undo_command_manager.push(Box::new(SelectNew));
    undo_command_manager.push(Box::new(SelectedToCursorSneaky));
    undo_command_manager.push(Box::new(DragToolUndo));
}

pub fn create_default_node(
    live_graph: &Arc<RwLock<LiveGraph>>,
    node_type: NodeType,
) -> Result<Node> {
    let node_id = live_graph.write().map_err(|e| anyhow!("{}", e))?.new_id();
    Ok(Node::with_id(node_type, node_id)
        .resize_policy(vismut_core::node::ResizePolicy::MostPixels)
        .resize_filter(vismut_core::node::ResizeFilter::Triangle))
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
