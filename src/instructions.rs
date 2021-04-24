use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

use crate::{camera::FirstPersonState, GrabToolType, Stage, ToolState};

const START_INSTRUCT: &str = &"Shift A: Add node";

struct Instructions;

pub(crate) struct InstructionPlugin;

impl Plugin for InstructionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(update_instructions.system()),
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
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
