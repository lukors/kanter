use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use vismut_core::{live_graph::LiveGraph, node::ResizePolicy, slot_data::Size as TPSize};

use crate::{
    core_translation::Translator,
    instruction::*,
    mouse_interaction::active::Active,
    shared::NodeIdComponent,
    undo::{gui::GuiUndoCommand, prelude::*},
};

use super::EditState;

pub(super) fn edit_specific_size_update(
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
            if event.char.is_digit(10) || event.char == 'x' {
                instructions.sections[1].value.push(event.char);
            } else if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let (Ok(from), Some(size)) = (
                    node_id.0.get(&*live_graph.read().unwrap()),
                    string_to_size(&instructions.sections[1].value),
                ) {
                    undo_command_manager.push(Box::new(GuiUndoCommand::new(
                        node_id.0,
                        from,
                        ResizePolicy::SpecificSize(size),
                    )));
                    undo_command_manager.push(Box::new(Checkpoint));
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

pub(super) fn edit_specific_size_enter(
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) =
        (q_active.get_single(), q_instructions.get_single_mut())
    {
        if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
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
