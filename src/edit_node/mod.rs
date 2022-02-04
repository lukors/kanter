mod name;
mod size;
mod slot;
mod value;

use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use vismut_core::{
    live_graph::LiveGraph,
    node::{mix::MixType, node_type::NodeType, Node, ResizeFilter, ResizePolicy},
};

use crate::{
    core_translation::Translator,
    instruction::*,
    listable::*,
    mouse_interaction::active::Active,
    scan_code_input::*,
    shared::NodeIdComponent,
    undo::{gui::GuiUndoCommand, prelude::*},
    AmbiguitySet, CustomStage, ToolState,
};

use self::{
    name::{edit_name_enter, edit_name_update},
    size::{edit_specific_size_enter, edit_specific_size_update},
    slot::{edit_specific_slot_enter, edit_specific_slot_update},
    value::{edit_value_enter, edit_value_update},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum EditState {
    None,
    Outer,
    Inner,
    Size,
    Slot,
    Value,
    Name,
}

#[derive(Clone, Debug)]
enum EditTarget {
    ResizePolicy,
    ResizeFilter,
    MixType,
}

type OptionEditTarget = Option<EditTarget>;

pub(crate) struct EditNodePlugin;

impl Plugin for EditNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_state(EditState::None)
            .insert_resource(OptionEditTarget::default())
            .add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(CustomStage::Update)
                    .after(CustomStage::Setup)
                    .in_ambiguity_set(AmbiguitySet)
                    .with_system(
                        tool_enter
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::EditNode)),
                    )
                    .with_system(
                        tool_exit
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::EditNode)),
                    )
                    .with_system(
                        edit_exit
                            .system()
                            .with_run_criteria(State::on_enter(EditState::Outer)),
                    )
                    .with_system(
                        tool_update
                            .system()
                            .with_run_criteria(State::on_update(EditState::Outer)),
                    )
                    .with_system(
                        edit.system()
                            .with_run_criteria(State::on_update(EditState::Inner)),
                    )
                    .with_system(
                        edit_specific_size_enter
                            .system()
                            .with_run_criteria(State::on_enter(EditState::Size)),
                    )
                    .with_system(
                        edit_specific_size_update
                            .system()
                            .with_run_criteria(State::on_update(EditState::Size)),
                    )
                    .with_system(
                        edit_specific_slot_enter
                            .system()
                            .with_run_criteria(State::on_enter(EditState::Slot)),
                    )
                    .with_system(
                        edit_specific_slot_update
                            .system()
                            .with_run_criteria(State::on_update(EditState::Slot)),
                    )
                    .with_system(
                        edit_value_enter
                            .system()
                            .with_run_criteria(State::on_enter(EditState::Value)),
                    )
                    .with_system(
                        edit_value_update
                            .system()
                            .with_run_criteria(State::on_update(EditState::Value)),
                    )
                    .with_system(
                        edit_name_enter
                            .system()
                            .with_run_criteria(State::on_enter(EditState::Name)),
                    )
                    .with_system(
                        edit_name_update
                            .system()
                            .with_run_criteria(State::on_update(EditState::Name)),
                    ),
            );
    }
}

fn setup(mut tool_list: ResMut<ToolList>) {
    tool_list.insert("Tab: Edit node".to_string());
}

fn tool_update(
    mut edit_state: ResMut<State<EditState>>,
    mut tool_state: ResMut<State<ToolState>>,
    mut scan_code_input: ResMut<ScanCodeInput>,
    mut instructions: ResMut<Instructions>,
    mut edit_target: ResMut<OptionEditTarget>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    let active_id = if let Ok(node_id) = q_active.get_single() {
        if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
            show_instructions(&node, &mut instructions);
        } else {
            tool_state.overwrite_replace(ToolState::None).unwrap();
            return;
        }
        node_id.0
    } else {
        tool_state.overwrite_replace(ToolState::None).unwrap();
        return;
    };

    let node_type = if let Ok(node) = live_graph.read().unwrap().node(active_id) {
        node.node_type
    } else {
        tool_state.overwrite_replace(ToolState::None).unwrap();
        return;
    };

    let scan_codes: Vec<ScanCode> = scan_code_input.get_just_pressed().copied().collect();

    for scan_code in scan_codes {
        if match scan_code {
            ScanCode::KeyN => {
                if let NodeType::OutputRgba(_) = node_type {
                    edit_state.overwrite_replace(EditState::Name).unwrap();
                    true
                } else {
                    false
                }
            }
            ScanCode::Tab => {
                tool_state.overwrite_replace(ToolState::None).unwrap();
                true
            }
            ScanCode::KeyR => {
                instructions.insert(InstructId::Tool, ResizePolicy::list());
                *edit_target = Some(EditTarget::ResizePolicy);
                edit_state.overwrite_replace(EditState::Inner).unwrap();
                true
            }
            ScanCode::KeyF => {
                instructions.insert(InstructId::Tool, ResizeFilter::list());
                *edit_target = Some(EditTarget::ResizeFilter);
                edit_state.overwrite_replace(EditState::Inner).unwrap();
                true
            }
            ScanCode::KeyT => {
                if let NodeType::Mix(_) = node_type {
                    instructions.insert(InstructId::Tool, MixType::list());
                    *edit_target = Some(EditTarget::MixType);
                    edit_state.overwrite_replace(EditState::Inner).unwrap();
                    true
                } else {
                    false
                }
            }
            ScanCode::KeyV => {
                if let NodeType::Value(_) = node_type {
                    edit_state.overwrite_replace(EditState::Value).unwrap();
                    true
                } else {
                    false
                }
            }
            _ => false,
        } {
            scan_code_input.clear_just_pressed(scan_code);
            break;
        }
    }
}

fn edit(
    mut edit_state: ResMut<State<EditState>>,
    mut scan_code_input: ResMut<ScanCodeInput>,
    mut edit_target: ResMut<OptionEditTarget>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    let mut done = false;

    if let (Some(edit_target), Ok(node_id)) = (&*edit_target, q_active.get_single()) {
        if let Ok(live_graph) = live_graph.read() {
            let scan_codes: Vec<ScanCode> = scan_code_input.get_just_pressed().copied().collect();
            let mut parameter_set = false;

            for scan_code in scan_codes {
                if let Some(i) = scan_code.to_usize() {
                    match edit_target {
                        EditTarget::ResizePolicy => match ResizePolicy::choose(i) {
                            Some(ResizePolicy::SpecificSize(_)) => {
                                edit_state.overwrite_replace(EditState::Size).unwrap();
                                return;
                            }
                            Some(ResizePolicy::SpecificSlot(_)) => {
                                edit_state.overwrite_replace(EditState::Slot).unwrap();
                                return;
                            }
                            Some(to) => {
                                if let Ok(from) = node_id.0.get(&*live_graph) {
                                    undo_command_manager
                                        .push(Box::new(GuiUndoCommand::new(node_id.0, from, to)));
                                    undo_command_manager.push(Box::new(Checkpoint));
                                    parameter_set = true;
                                }
                            }
                            None => (),
                        },
                        EditTarget::ResizeFilter => {
                            if let (Ok(from), Some(to)) =
                                (node_id.0.get(&*live_graph), ResizeFilter::choose(i))
                            {
                                undo_command_manager
                                    .push(Box::new(GuiUndoCommand::new(node_id.0, from, to)));
                                undo_command_manager.push(Box::new(Checkpoint));
                                parameter_set = true;
                            }
                        }
                        EditTarget::MixType => {
                            if let Some(mix_type) = MixType::choose(i) {
                                if let Ok(from) = node_id.0.get(&*live_graph) {
                                    undo_command_manager.push(Box::new(GuiUndoCommand::new(
                                        node_id.0,
                                        from,
                                        NodeType::Mix(mix_type),
                                    )));
                                    undo_command_manager.push(Box::new(Checkpoint));
                                    parameter_set = true;
                                } else {
                                    error!("unable to get node with id: {}", node_id.0);
                                }
                            }
                        }
                    }

                    if parameter_set {
                        scan_code_input.clear_just_pressed(scan_code);
                        done = true;
                        break;
                    }
                } else if scan_code == ScanCode::Tab {
                    scan_code_input.clear_just_pressed(scan_code);
                    done = true;
                    break;
                }
            }
        }
    }

    if done {
        *edit_target = None;
        edit_state.overwrite_replace(EditState::Outer).unwrap();
    }
}

fn edit_exit(
    q_active: Query<&NodeIdComponent, With<Active>>,
    mut instructions: ResMut<Instructions>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let Ok(node_id) = q_active.get_single() {
        if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
            show_instructions(&node, &mut instructions);
        } else {
            error!("Could not find a node with that ID in the graph");
        }
    }
}

fn show_instructions(node: &Node, instructions: &mut Instructions) {
    let generic_instructions = format!(
        "R: Resize policy ({})\nF: Resize filter ({})",
        node.resize_policy, node.resize_filter
    );

    let specific_instructions = {
        if let Some(name) = node.node_type.name() {
            format!("N: Name: {}", name)
        } else {
            match &node.node_type {
                NodeType::Image(path) => format!("Path: {:#?}", path),
                NodeType::Mix(mix_type) => format!("T: Type: {}", mix_type),
                NodeType::Value(value) => format!("V: Value: {}", value),
                _ => "Unsupported node".to_string(),
            }
        }
    };

    instructions.insert(
        InstructId::Tool,
        format!(
            "# {}\n{}\n\n{}",
            node_type_name(&node.node_type),
            specific_instructions,
            generic_instructions
        ),
    );
}

fn tool_enter(
    mut edit_state: ResMut<State<EditState>>,
    mut tool_state: ResMut<State<ToolState>>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut instructions: ResMut<Instructions>,
) {
    if let Ok(node_id) = q_active.get_single() {
        if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
            let _ = edit_state.overwrite_replace(EditState::Outer);

            show_instructions(&node, &mut instructions);
        } else {
            error!("Could not find a node with that ID in the graph");
            tool_state.overwrite_replace(ToolState::None).unwrap();
        }
    } else {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

fn tool_exit(mut edit_state: ResMut<State<EditState>>) {
    let _ = edit_state.overwrite_replace(EditState::None);
}

fn node_type_name(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::InputGray(_) => "InputGray",
        NodeType::InputRgba(_) => "InputRgba",
        NodeType::OutputGray(_) => "OutputGray",
        NodeType::OutputRgba(_) => "OutputRgba",
        NodeType::Graph(_) => "Graph",
        NodeType::Image(_) => "Image",
        NodeType::Embed(_) => "Embedded Image",
        NodeType::Write(_) => "Write",
        NodeType::Value(_) => "Value",
        NodeType::Mix(_) => "Mix",
        NodeType::HeightToNormal => "Height To Normal",
        NodeType::SeparateRgba => "Separate RGBA",
        NodeType::CombineRgba => "Combine RGBA",
    }
}
