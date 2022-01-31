use std::sync::{Arc, RwLock};

use anyhow::Result;
use bevy::prelude::*;
use kanter_core::{live_graph::LiveGraph, slot_data::ChannelPixel};

use crate::{
    core_translation::Translator,
    instruction::*,
    mouse_interaction::active::Active,
    shared::NodeIdComponent,
    undo::{gui::GuiUndoCommand, prelude::*},
};

use super::EditState;

fn edit_value_display(instructions: &mut Text, value: f32) {
    instructions.sections[0].value = format!("Current value: {}\nNew: ", value);
    instructions.sections[1].value.clear();
}

pub(super) fn edit_value_enter(
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) =
        (q_active.get_single(), q_instructions.get_single_mut())
    {
        if let Ok(live_graph) = live_graph.read() {
            let value: Result<ChannelPixel> = node_id.0.get(&*live_graph);
            if let Ok(value) = value {
                edit_value_display(&mut instructions, value);
            }
        }
    }
}

pub(super) fn edit_value_update(
    mut char_input_events: EventReader<ReceivedCharacter>,
    mut edit_state: ResMut<State<EditState>>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    mut started: Local<bool>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    // This guard drops any input the first time the system is entered, so you do not get the
    // input from the button that was pressed to start this sytem, in this sytem.
    if !*started {
        *started = true;
        return;
    }

    if let (Ok(mut instructions), Ok(node_id)) =
        (q_instructions.get_single_mut(), q_active.get_single())
    {
        for event in char_input_events.iter() {
            if event.char.is_digit(10) || event.char == '.' {
                instructions.sections[1].value.push(event.char);
            } else if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let Ok(live_graph) = live_graph.read() {
                    if let (Ok(number), Ok(previous)) = (
                        instructions.sections[1].value.parse::<f32>(),
                        node_id.0.get(&*live_graph),
                    ) {
                        let gui_translator = GuiUndoCommand::new(node_id.0, previous, number);
                        undo_command_manager.push(Box::new(gui_translator));
                        undo_command_manager.push(Box::new(Checkpoint));
                    } else {
                        warn!("Invalid number format, should be for instance 0.3");
                    }
                }
                edit_state.overwrite_replace(EditState::Outer).unwrap();
                *started = false;
            }
        }
    }
}
