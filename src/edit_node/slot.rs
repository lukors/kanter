use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use vismut_core::{live_graph::LiveGraph, node::ResizePolicy};

use crate::{
    core_translation::Translator,
    instruction::*,
    mouse_interaction::active::Active,
    shared::NodeIdComponent,
    undo::{gui::GuiUndoCommand, prelude::*},
};

use super::EditState;

pub(super) fn edit_specific_slot_enter(
    mut edit_state: ResMut<State<EditState>>,
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) =
        (q_active.get_single(), q_instructions.get_single_mut())
    {
        if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
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

pub(super) fn edit_specific_slot_update(
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
            if event.char.is_digit(10) {
                instructions.sections[1].value.push(event.char);
            } else if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let Ok(index) = instructions.sections[1].value.parse::<u32>() {
                    if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
                        if let (Ok(from), Some(slot)) = (
                            node_id.0.get(&*live_graph.read().unwrap()),
                            node.input_slots().get(index as usize),
                        ) {
                            let slot_id = (*slot).slot_id;
                            if node.input_slot_with_id(slot_id).is_ok() {
                                undo_command_manager.push(Box::new(GuiUndoCommand::new(
                                    node_id.0,
                                    from,
                                    ResizePolicy::SpecificSlot(slot_id),
                                )));
                                undo_command_manager.push(Box::new(Checkpoint));
                            } else {
                                warn!("Node does not have a slot with the given ID: {}", slot_id);
                            }
                        } else {
                            warn!("That slot does not exist: {}", index);
                        }
                    } else {
                        error!(
                            "The node you're trying to edit does not exist: {}",
                            node_id.0
                        );
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
