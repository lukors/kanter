/// All workspace mouse interaction.
use crate::{Drag, Dropped, GrabToolType, Hovered, Selected, Slot, ToolState, Workspace, WorkspaceCamera};
use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

/// Handles all mouse clicks and drags in the workspace. Like dragging nodes and box select.
#[allow(clippy::too_many_arguments)]
pub(crate) fn mouse_interaction(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_hovered_node: Query<Entity, (With<NodeId>, With<Hovered>)>,
    q_selected_node: Query<Entity, (With<NodeId>, With<Selected>)>,
    q_hovered_selected_node: Query<Entity, (With<NodeId>, With<Selected>, With<Hovered>)>,
    q_hovered_slot: Query<Entity, (With<Slot>, With<Hovered>)>,
    q_selected_slot: Query<Entity, (With<Slot>, With<Selected>)>,
    q_selected: Query<Entity, With<Selected>>,
    q_dropped: Query<&Dropped>,
    workspace: Res<Workspace>,
) {
    let some_dropped = q_dropped.iter().count() > 0;
    let single_click = i_mouse_button.just_released(MouseButton::Left)
        && workspace.drag != Drag::Dropping
        && !some_dropped;

    if single_click {
        // Deselect everything.
        for entity in q_selected.iter() {
            commands.entity(entity).remove::<Selected>();
        }
    }

    if let Some(entity) = q_hovered_slot.iter().next() {
        // Slot
        if single_click {
            // Select the one slot
            commands.entity(entity).insert(Selected);
        } else if workspace.drag == Drag::Starting {
            // Drag on slot
            for entity in q_selected_slot.iter() {
                commands.entity(entity).remove::<Selected>();
            }
            commands.entity(entity).insert(Selected);
            tool_state.overwrite_replace(ToolState::Grab(GrabToolType::Slot)).unwrap();
        }
    } else if let Some(entity) = q_hovered_node.iter().next() {
        // Node
        if single_click {
            // Select the one node
            commands.entity(entity).insert(Selected);
        } else if workspace.drag == Drag::Starting {
            // Drag on node
            let some_hovered_selected_node = q_hovered_selected_node.iter().count() > 0;
            if some_hovered_selected_node {
                tool_state.overwrite_replace(ToolState::Grab(GrabToolType::Node)).unwrap();
            } else {
                for entity in q_selected_node.iter() {
                    commands.entity(entity).remove::<Selected>();
                }

                commands.entity(entity).insert(Selected);
                tool_state.overwrite_replace(ToolState::Grab(GrabToolType::Node)).unwrap();
            }
        }
    } else if workspace.drag == Drag::Starting {
        // Drag on empty space
        tool_state.overwrite_replace(ToolState::BoxSelect).unwrap();
    }
}

/// Pan using the mouse.
pub(crate) fn mouse_pan(
    mut windows: ResMut<Windows>,
    workspace: Res<Workspace>,
    mut camera: Query<&mut Transform, With<WorkspaceCamera>>,
    i_mouse_button: Res<Input<MouseButton>>,
    mut frozen_cursor_pos: Local<Vec2>,
) {
    if i_mouse_button.just_pressed(MouseButton::Middle) {
        *frozen_cursor_pos = workspace.cursor_screen;
    }
    if i_mouse_button.pressed(MouseButton::Middle) && workspace.cursor_moved {
        let window = windows.get_primary_mut().unwrap();
        window.set_cursor_position(*frozen_cursor_pos);

        if let Ok(mut camera_t) = camera.single_mut() {
            camera_t.translation.x += workspace.cursor_delta.x;
            camera_t.translation.y -= workspace.cursor_delta.y;
        }
    }
}