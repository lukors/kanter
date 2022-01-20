use std::collections::BTreeSet;

use bevy::{prelude::*, utils::HashMap};

use crate::{AmbiguitySet, Stage, ToolState};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum InstructId {
    FirstPerson,
    Tool,
}

#[derive(Debug, Default)]
pub(crate) struct ToolList(BTreeSet<String>);

impl std::ops::Deref for ToolList {
    type Target = BTreeSet<String>;
    fn deref(&self) -> &BTreeSet<String> {
        &self.0
    }
}

impl std::ops::DerefMut for ToolList {
    fn deref_mut(&mut self) -> &mut BTreeSet<String> {
        &mut self.0
    }
}

#[derive(Component)]
pub(crate) struct InstructionMarker;

#[derive(Default)]
pub(crate) struct Instructions(pub HashMap<InstructId, String>);

impl std::ops::Deref for Instructions {
    type Target = HashMap<InstructId, String>;
    fn deref(&self) -> &HashMap<InstructId, String> {
        &self.0
    }
}

impl std::ops::DerefMut for Instructions {
    fn deref_mut(&mut self) -> &mut HashMap<InstructId, String> {
        &mut self.0
    }
}
pub(crate) struct InstructionPlugin;

impl Plugin for InstructionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send_resource(ToolList::default())
            .insert_non_send_resource(Instructions::default())
            .add_startup_system(setup.system().in_ambiguity_set(AmbiguitySet))
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Apply)
                    .after(Stage::Update)
                    .with_system(update_instructions.system().in_ambiguity_set(AmbiguitySet)),
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut text = Text::default();
    text.sections.resize(
        2,
        TextSection {
            style: TextStyle {
                font: asset_server.load("fonts/FiraSans-Regular.ttf"),
                font_size: 20.0,
                color: Color::WHITE,
            },
            ..Default::default()
        },
    );
    text.sections[InstructId::Tool as usize].value =
        "Welcome to Kanter!\n\nShift A: Add node\n".to_string();

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
                    text,
                    ..Default::default()
                })
                .insert(InstructionMarker);
        });
}

fn update_instructions(
    tool_state: Res<State<ToolState>>,
    instructions: Res<Instructions>,
    tool_list: Res<ToolList>,
    mut q_instruction_text: Query<&mut Text, With<InstructionMarker>>,
) {
    if instructions.is_changed() && !instructions.is_added() {
        if let Ok(mut text) = q_instruction_text.single_mut() {
            for text_section in text.sections.iter_mut() {
                text_section.value.clear();
            }

            for (key, text_value) in instructions.iter() {
                text.sections[*key as usize].value = text_value.to_string();
            }

            if *tool_state.current() == ToolState::None {
                let mut tool_list_text = String::new();

                for entry in tool_list.iter() {
                    tool_list_text = format!("{}{}\n", tool_list_text, entry);
                }

                text.sections[InstructId::Tool as usize].value = tool_list_text;
            }
        }
    }
}
