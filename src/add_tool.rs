/// Adding new nodes
use crate::{
    drag_drop_entity::{grab_tool_cleanup, grab_tool_node_setup},
    instruction::*,
    AmbiguitySet, GrabToolType, Stage, ToolState,
};
use bevy::prelude::*;
use kanter_core::{
    error::TexProError,
    node::{MixType, Node, NodeType},
    node_graph::NodeId,
    texture_processor::TextureProcessor,
};
use native_dialog::FileDialog;

pub(crate) struct AddToolPlugin;

impl Plugin for AddToolPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Setup)
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
        "C: Combine\nI: Input\nM: Mix\nO: Output\nV: Value\nS: Separate\n".to_string(),
    );
}

/// When you press the button for a node it creates that node for you.
fn add_update(
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut tool_state: ResMut<State<ToolState>>,
    mut tex_pro: ResMut<TextureProcessor>,
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
            if create_default_node(&mut tex_pro, node_type.clone()).is_ok() {
                info!("Added node: {:?}", node_type);
            }
            tool_state
                .overwrite_replace(ToolState::Grab(GrabToolType::Add))
                .unwrap();
            break;
        } else if done {
            tool_state.overwrite_replace(ToolState::None).unwrap();
            break;
        }
    }
}

fn create_default_node(
    tex_pro: &mut TextureProcessor,
    node_type: NodeType,
) -> Result<NodeId, TexProError> {
    tex_pro.add_node(
        Node::new(node_type)
            .resize_policy(kanter_core::node::ResizePolicy::MostPixels)
            .resize_filter(kanter_core::node::ResizeFilter::Triangle),
    )
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
