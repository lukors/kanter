use bevy::prelude::*;
use kanter_core::{
    dag::TextureProcessor,
    node::{Node, NodeType, ResizeFilter, ResizePolicy},
    node_data::Size as TPSize,
    node_graph::{NodeId, SlotId},
};

use crate::{
    instruction::*, mouse_interaction::Active, scan_code_input::*, AmbiguitySet, Stage, ToolState,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum EditState {
    Outer,
    Inner,
}

#[derive(Clone, Debug)]
enum EditTarget {
    ResizePolicy,
    ResizeFilter,
}

type OptionEditTarget = Option<EditTarget>;

pub(crate) struct EditNodePlugin;

impl Plugin for EditNodePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_state(EditState::Outer)
            .insert_resource(OptionEditTarget::default())
            .add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Update)
                    .after(Stage::Input)
                    .in_ambiguity_set(AmbiguitySet)
                    .with_system(
                        tool_enter
                            .system()
                            .with_run_criteria(State::on_enter(ToolState::EditNode)),
                    )
                    .with_system(
                        tool_update
                            .system()
                            .with_run_criteria(State::on_update(ToolState::EditNode))
                            .with_run_criteria(State::on_update(EditState::Outer)),
                    )
                    .with_system(
                        edit.system()
                            .with_run_criteria(State::on_update(ToolState::EditNode))
                            .with_run_criteria(State::on_update(EditState::Inner)),
                    )
                    .with_system(
                        edit_exit
                            .system()
                            .with_run_criteria(State::on_update(ToolState::EditNode))
                            .with_run_criteria(State::on_exit(EditState::Inner)),
                    )
                    .with_system(
                        tool_exit
                            .system()
                            .with_run_criteria(State::on_exit(ToolState::EditNode)),
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
                true
            }
            ScanCode::KeyF => {
                instructions.insert(InstructId::Tool, ResizeFilter::list());
                *edit_target = Some(EditTarget::ResizeFilter);
                true
            }
            _ => false,
        } {
            edit_state.overwrite_replace(EditState::Inner).unwrap();
            scan_code_input.clear_just_pressed(scan_code);
            break;
        }
    }
}

fn edit(
    mut edit_state: ResMut<State<EditState>>,
    mut scan_code_input: ResMut<ScanCodeInput>,
    mut edit_target: ResMut<OptionEditTarget>,
    q_active: Query<&NodeId, With<Active>>,
    mut tex_pro: ResMut<TextureProcessor>,
) {
    let mut valid_input = None;

    if let (Some(edit_target), Ok(node_id)) = (&*edit_target, q_active.single()) {
        if let Some(node) = tex_pro.node_graph.node_with_id_mut(*node_id) {
            let scan_codes: Vec<ScanCode> = scan_code_input.get_just_pressed().copied().collect();

            for scan_code in scan_codes {
                if let Some(i) = scan_code.to_usize() {
                    match edit_target {
                        EditTarget::ResizePolicy => {
                            if let Some(resize_policy) = ResizePolicy::choose(i) {
                                node.resize_policy = resize_policy;
                            }
                        }
                        EditTarget::ResizeFilter => {
                            if let Some(resize_filter) = ResizeFilter::choose(i) {
                                node.resize_filter = resize_filter;
                            }
                        }
                    }
                    valid_input = Some(scan_code);
                    break;
                } else if scan_code == ScanCode::Tab {
                    valid_input = Some(scan_code);
                    break;
                }
            }
        }
    }

    if let Some(scan_code) = valid_input {
        scan_code_input.clear_just_pressed(scan_code);
        *edit_target = None;
        edit_state.overwrite_replace(EditState::Outer).unwrap();
    }
}

fn edit_exit(
    q_active: Query<&NodeId, With<Active>>,
    mut instructions: ResMut<Instructions>,
    tex_pro: ResMut<TextureProcessor>,
) {
    if let Ok(node_id) = q_active.single() {
        if let Some(node) = tex_pro.node_graph.node_with_id(*node_id) {
            show_instructions(node, &mut instructions);
        } else {
            error!("Could not find a node with that ID in the graph");
        }
    }
}

fn tool_exit(mut instructions: ResMut<Instructions>) {
    instructions.remove(&InstructId::Tool);
}

fn show_instructions(node: &Node, instructions: &mut Instructions) {
    let generic_instructions = format!(
        "R: Resize policy ({})\nF: Resize filter ({})",
        node.resize_policy, node.resize_filter
    );

    let specific_instructions = match &node.node_type {
        NodeType::Image(path) => format!("Path: {:#?}", path),
        _ => "Unsupported node".to_string(),
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
    tex_pro: Res<TextureProcessor>,
    mut instructions: ResMut<Instructions>,
) {
    if let Ok(node_id) = q_active.single() {
        if let Some(node) = tex_pro.node_graph.node_with_id(*node_id) {
            let _ = edit_state.overwrite_replace(EditState::Outer);

            show_instructions(node, &mut instructions);
        } else {
            error!("Could not find a node with that ID in the graph");
            tool_state.overwrite_replace(ToolState::None).unwrap();
        }
    } else {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

trait Listable<T> {
    fn list() -> String;
    fn choose(i: usize) -> Option<T>;
}

impl Listable<Self> for ResizePolicy {
    fn list() -> String {
        let mut output = "## Resize policy\n".to_string();
        let entries = vec![
            "MostPixels".to_string(),
            "LeastPixels".to_string(),
            "LargestAxes".to_string(),
            "SmallestAxes".to_string(),
            "SpecificSlot".to_string(),
            "SpecificSize".to_string(),
        ];
        for (i, entry) in entries.iter().enumerate() {
            output = format!("{}{}: {}\n", output, i + 1, entry);
        }
        output
    }

    fn choose(i: usize) -> Option<Self> {
        const MAX_CHOICE: usize = 6;

        if i <= MAX_CHOICE {
            Some(match i {
                1 => Self::MostPixels,
                2 => Self::LeastPixels,
                3 => Self::LargestAxes,
                4 => Self::SmallestAxes,
                5 => Self::SpecificSlot(SlotId(0)),
                _ => Self::SpecificSize(TPSize::new(128, 128)),
            })
        } else {
            None
        }
    }
}

impl Listable<Self> for ResizeFilter {
    fn list() -> String {
        let mut output = "## Resize filter\n".to_string();
        let entries = vec!["Nearest".to_string(), "Triangle".to_string()];
        for (i, entry) in entries.iter().enumerate() {
            output = format!("{}{}: {}\n", output, i + 1, entry);
        }
        output
    }

    fn choose(i: usize) -> Option<Self> {
        const MAX_CHOICE: usize = 2;

        if i <= MAX_CHOICE {
            Some(match i {
                1 => Self::Nearest,
                _ => Self::Triangle,
            })
        } else {
            None
        }
    }
}

fn node_type_name(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::InputGray => "InputGray",
        NodeType::InputRgba => "InputRgba",
        NodeType::OutputGray => "OutputGray",
        NodeType::OutputRgba => "OutputRgba",
        NodeType::Graph(_) => "Graph",
        NodeType::Image(_) => "Image",
        NodeType::NodeData(_) => "Embedded Image",
        NodeType::Write(_) => "Write",
        NodeType::Value(_) => "Value",
        NodeType::Mix(_) => "Mix",
        NodeType::HeightToNormal => "Height To Normal",
    }
}
