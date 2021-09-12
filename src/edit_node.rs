use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use kanter_core::{live_graph::LiveGraph, node::{mix::MixType, node_type::NodeType, Node, ResizeFilter, ResizePolicy}, node_graph::NodeId, slot_data::Size as TPSize};

use crate::{
    instruction::*, listable::*, mouse_interaction::Active, scan_code_input::*, AmbiguitySet,
    Stage, ToolState,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum EditState {
    None,
    Outer,
    Inner,
    Size,
    Slot,
    Value,
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
    fn build(&self, app: &mut AppBuilder) {
        app.add_state(EditState::None)
            .insert_resource(OptionEditTarget::default())
            .add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Update)
                    .after(Stage::Setup)
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
                        tool_update
                            .system()
                            .with_run_criteria(State::on_update(EditState::Outer)),
                    )
                    .with_system(
                        edit.system()
                            .with_run_criteria(State::on_update(EditState::Inner)),
                    )
                    .with_system(
                        edit_exit
                            .system()
                            .with_run_criteria(State::on_enter(EditState::Outer)),
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
) {
    let scan_codes: Vec<ScanCode> = scan_code_input.get_just_pressed().copied().collect();

    for scan_code in scan_codes {
        if match scan_code {
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
                instructions.insert(InstructId::Tool, MixType::list());
                *edit_target = Some(EditTarget::MixType);
                edit_state.overwrite_replace(EditState::Inner).unwrap();
                true
            }
            ScanCode::KeyV => {
                edit_state.overwrite_replace(EditState::Value).unwrap();
                true
            }
            _ => false,
        } {
            scan_code_input.clear_just_pressed(scan_code);
            break;
        }
    }
}

fn edit_specific_slot_enter(
    mut edit_state: ResMut<State<EditState>>,
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) = (q_active.single(), q_instructions.single_mut()) {
        if let Ok(node) = live_graph.read().unwrap().node(*node_id) {
            if node.input_slots().is_empty() {
                warn!("The node doesn't have any input slots");
                edit_state.overwrite_set(EditState::Outer).unwrap();
                return;
            } else if let ResizePolicy::SpecificSlot(slot) = node.resize_policy {
                instructions.sections[0].value = format!("Current: {}\nNew: ", slot);
            } else {
                instructions.sections[0].value = format!(
                    "Available slots are 0 through {}\nChoice: ",
                    node.input_slots().len()
                );
            }
        }
        instructions.sections[1].value.clear();
    }
}

fn edit_specific_slot_update(
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut edit_state: ResMut<State<EditState>>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    mut started: Local<bool>,
) {
    // This guard drops any input the first time the system is entered, so you do not get the
    // input from the button that was pressed to start this sytem, in this sytem.
    if !*started {
        *started = true;
        return;
    }

    if let (Ok(mut instructions), Ok(node_id)) = (q_instructions.single_mut(), q_active.single()) {
        for event in char_input_events.iter() {
            if event.char.is_digit(10) {
                instructions.sections[1].value.push(event.char);
            } else if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let Ok(index) = instructions.sections[1].value.parse::<u32>() {
                    if let Ok(mut node) = live_graph.write().unwrap().node_mut(*node_id) {
                        if let Some(slot) = node.input_slots().get(index as usize) {
                            let slot_id = (*slot).slot_id;
                            if node.input_slot_with_id(slot_id).is_ok() {
                                node.resize_policy = ResizePolicy::SpecificSlot(slot_id);
                            } else {
                                warn!("Node does not have a slot with the given ID: {}", slot_id);
                            }
                        } else {
                            warn!("That slot does not exist: {}", index);
                        }
                    } else {
                        error!("The node you're trying to edit does not exist: {}", node_id);
                    }
                } else {
                    error!(
                        "Could not parse the input as a number: {}",
                        instructions.sections[1].value
                    );
                }
                edit_state.overwrite_replace(EditState::Outer).unwrap();
                *started = false;
            }
        }
    }
}

fn edit_specific_size_enter(
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) = (q_active.single(), q_instructions.single_mut()) {
        if let Ok(node) = live_graph.read().unwrap().node(*node_id) {
            if let ResizePolicy::SpecificSize(size) = node.resize_policy {
                instructions.sections[0].value =
                    format!("Current: {}x{}\nNew: ", size.width, size.height);
            } else {
                instructions.sections[0].value = "Example format: 256x256\nNew: ".to_string();
            }
        }
        instructions.sections[1].value.clear();
    }
}

fn edit_specific_size_update(
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut edit_state: ResMut<State<EditState>>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    mut started: Local<bool>,
) {
    // This guard drops any input the first time the system is entered, so you do not get the
    // input from the button that was pressed to start this sytem, in this sytem.
    if !*started {
        *started = true;
        return;
    }

    if let (Ok(mut instructions), Ok(node_id)) = (q_instructions.single_mut(), q_active.single()) {
        for event in char_input_events.iter() {
            if event.char.is_digit(10) || event.char == 'x' {
                instructions.sections[1].value.push(event.char);
            } else if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let (Some(size), Ok(mut node)) = (
                    string_to_size(&instructions.sections[1].value),
                    live_graph.write().unwrap().node_mut(*node_id),
                ) {
                    node.resize_policy = ResizePolicy::SpecificSize(size);
                } else {
                    warn!("Invalid size format, should be for instance 256x256");
                }
                edit_state.overwrite_replace(EditState::Outer).unwrap();
                *started = false;
            }
        }
    }
}

fn string_to_size(input: &str) -> Option<TPSize> {
    let sizes: Vec<&str> = input.split('x').collect();
    if sizes.len() == 2 {
        if let (Ok(width), Ok(height)) = (sizes[0].parse(), sizes[1].parse()) {
            Some(TPSize::new(width, height))
        } else {
            None
        }
    } else {
        None
    }
}

fn edit_value_enter(
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) = (q_active.single(), q_instructions.single_mut()) {
        if let Ok(node) = live_graph.read().unwrap().node(*node_id) {
            if let NodeType::Value(value) = node.node_type {
                instructions.sections[0].value =
                    format!("Current value: {}\nNew: ", value.to_string());
            }
            instructions.sections[1].value.clear();
        }
    }
}

fn edit_value_update(
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut edit_state: ResMut<State<EditState>>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    mut started: Local<bool>,
) {
    // This guard drops any input the first time the system is entered, so you do not get the
    // input from the button that was pressed to start this sytem, in this sytem.
    if !*started {
        *started = true;
        return;
    }

    if let (Ok(mut instructions), Ok(node_id)) = (q_instructions.single_mut(), q_active.single()) {
        for event in char_input_events.iter() {
            if event.char.is_digit(10) || event.char == '.' {
                instructions.sections[1].value.push(event.char);
            } else if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let (Ok(number), Ok(mut node)) = (
                    instructions.sections[1].value.parse::<f32>(),
                    live_graph.write().unwrap().node_mut(*node_id),
                ) {
                    node.node_type = NodeType::Value(number);
                } else {
                    warn!("Invalid number format, should be for instance 0.3");
                }
                edit_state.overwrite_replace(EditState::Outer).unwrap();
                *started = false;
            }
        }
    }
}

fn edit(
    mut edit_state: ResMut<State<EditState>>,
    mut scan_code_input: ResMut<ScanCodeInput>,
    mut edit_target: ResMut<OptionEditTarget>,
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    let mut done = false;

    if let (Some(edit_target), Ok(node_id)) = (&*edit_target, q_active.single()) {
        if let Ok(mut engine) = live_graph.write() {
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
                            Some(resize_policy) => {
                                if let Ok(node) = engine.node_mut(*node_id) {
                                    node.resize_policy = resize_policy;
                                    parameter_set = true;
                                } else {
                                    error!("Unable to get node with id: {}", node_id);
                                }
                            }
                            None => (),
                        },
                        EditTarget::ResizeFilter => {
                            if let Some(resize_filter) = ResizeFilter::choose(i) {
                                if let Ok(node) = engine.node_mut(*node_id) {
                                    node.resize_filter = resize_filter;
                                    parameter_set = true;
                                } else {
                                    error!("Unable to get node with id: {}", node_id);
                                }
                            }
                        }
                        EditTarget::MixType => {
                            if let Some(mix_type) = MixType::choose(i) {
                                if let Ok(node) = engine.node_mut(*node_id) {
                                    node.node_type = NodeType::Mix(mix_type);
                                    parameter_set = true;
                                } else {
                                    error!("Unable to get node with id: {}", node_id);
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
    q_active: Query<&NodeId, With<Active>>,
    mut instructions: ResMut<Instructions>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let Ok(node_id) = q_active.single() {
        if let Ok(node) = live_graph.read().unwrap().node(*node_id) {
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
    q_active: Query<&NodeId, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut instructions: ResMut<Instructions>,
) {
    if let Ok(node_id) = q_active.single() {
        if let Ok(node) = live_graph.read().unwrap().node(*node_id) {
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
    edit_state.overwrite_replace(EditState::None).unwrap();
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
