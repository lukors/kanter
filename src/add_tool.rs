/// Adding new nodes
use crate::{
    drag_drop_entity::{grab_tool_cleanup, grab_tool_node_setup},
    instruction::*,
    scan_code_input::{ScanCode, ScanCodeInput},
    AmbiguitySet, GrabToolType, Stage, ToolState,
};
use bevy::prelude::*;
use kanter_core::{
    dag::TextureProcessor,
    error::TexProError,
    node::{MixType, Node, NodeType},
    node_graph::NodeId,
};
use native_dialog::FileDialog;

pub(crate) struct AddToolPlugin;

impl Plugin for AddToolPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Update)
                    .after(Stage::Input)
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
                    )
                    .with_system(
                        grab_tool_clear_instructions
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
    instructions.insert(InstructId::Tool, "I: Input\nO: Output\n".to_string());
}

/// When you press the button for a node it creates that node for you.
fn add_update(
    mut scan_code_input: ResMut<ScanCodeInput>,
    mut tool_state: ResMut<State<ToolState>>,
    mut tex_pro: ResMut<TextureProcessor>,
) {
    let mut events_maybe_missed = false;

    for input in scan_code_input.get_just_pressed() {
        let node_type: Option<NodeType> = match input {
            ScanCode::KeyI => {
                events_maybe_missed = true;

                match FileDialog::new()
                    // .set_location("~/Desktop")
                    .add_filter("PNG Image", &["png"])
                    .add_filter("JPEG Image", &["jpg", "jpeg"])
                    .show_open_single_file()
                {
                    Ok(Some(path)) => Some(NodeType::Image(path)),
                    Ok(None) => {
                        warn!("Invalid path");
                        None
                    }
                    Err(e) => {
                        warn!("Error bringing up file dialog: {}", e);
                        None
                    }
                }
            }
            ScanCode::KeyM => Some(NodeType::Mix(MixType::Add)),
            ScanCode::KeyO => {
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
            if let Ok(_) = create_default_node(&mut tex_pro, node_type.clone()) {
                info!("Added node: {:?}", node_type);

                tool_state
                    .overwrite_replace(ToolState::Grab(GrabToolType::Add))
                    .unwrap();
                break;
            }
        }
    }

    if events_maybe_missed {
        scan_code_input.clear();
    }
}

fn create_default_node(
    tex_pro: &mut TextureProcessor,
    node_type: NodeType,
) -> Result<NodeId, TexProError> {
    tex_pro.node_graph.add_node(
        Node::new(node_type)
            .resize_policy(kanter_core::node::ResizePolicy::MostPixels)
            .resize_filter(kanter_core::node::ResizeFilter::Triangle),
    )
}

fn grab_tool_add_instructions(mut instructions: ResMut<Instructions>) {
    instructions.insert(InstructId::Tool, "LMB: Confirm\n".to_string());
}

fn grab_tool_clear_instructions(mut instructions: ResMut<Instructions>) {
    instructions.remove(&InstructId::Tool);
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
