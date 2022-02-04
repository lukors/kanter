use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use vismut_core::{live_graph::LiveGraph, node::node_type::NodeType};

use crate::{
    core_translation::Translator,
    instruction::*,
    mouse_interaction::active::Active,
    shared::NodeIdComponent,
    undo::{gui::GuiUndoCommand, prelude::*},
};

use super::EditState;

pub(super) fn edit_name_update(
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
            if event.char == '\u{8}' {
                // Backspace
                instructions.sections[1].value.pop();
            } else if event.char == '\r' {
                // Enter
                if let Ok(from) = node_id.0.get(&*live_graph.read().unwrap()) {
                    let name = instructions.sections[1].value.clone();
                    undo_command_manager.push(Box::new(GuiUndoCommand::new(node_id.0, from, name)));
                    undo_command_manager.push(Box::new(Checkpoint));
                } else {
                    warn!("Invalid name");
                }
                edit_state.overwrite_replace(EditState::Outer).unwrap();
                *started = false;
            } else {
                instructions.sections[1].value.push(event.char);
            }
        }
    }
}

pub(super) fn edit_name_enter(
    mut q_instructions: Query<&mut Text, With<InstructionMarker>>,
    q_active: Query<&NodeIdComponent, With<Active>>,
    live_graph: Res<Arc<RwLock<LiveGraph>>>,
) {
    if let (Ok(node_id), Ok(mut instructions)) =
        (q_active.get_single(), q_instructions.get_single_mut())
    {
        if let Ok(node) = live_graph.read().unwrap().node(node_id.0) {
            if let NodeType::OutputRgba(name) = node.node_type {
                instructions.sections[0].value = "Name: ".into();
                instructions.sections[1].value = name;
            } else {
                instructions.sections[0].value = "Name: ".to_string();
                instructions.sections[1].value.clear();
            }
        }
    }
}
