pub mod active;
pub mod select;

/// All workspace mouse interaction.
use crate::{
    shared::NodeIdComponent, undo::prelude::*, AmbiguitySet, CustomStage, Drag, Dropped,
    GrabToolType, Hovered, Slot, ToolState, Workspace,
};
use bevy::prelude::*;

use self::{
    active::{Active, MakeNodeActive, MakeNothingActive},
    select::{DeselectAll, ReplaceSelection, SelectNode, Selected},
};

pub(crate) struct MouseInteractionPlugin;

impl Plugin for MouseInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(CustomStage::Setup)
                .after(CustomStage::Input)
                .with_system(
                    mouse_interaction
                        .system()
                        .with_run_criteria(State::on_update(ToolState::None))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        );
    }
}

/// Handles all mouse clicks and drags in the workspace. Like dragging nodes and box select.
#[allow(clippy::too_many_arguments)]
fn mouse_interaction(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_hovered_node: Query<(Entity, &NodeIdComponent), With<Hovered>>,
    q_hovered_selected_node: Query<Entity, (With<NodeIdComponent>, With<Selected>, With<Hovered>)>,
    q_hovered_slot: Query<Entity, (With<Slot>, With<Hovered>)>,
    q_selected_slot: Query<Entity, (With<Slot>, With<Selected>)>,
    q_dropped: Query<&Dropped>,
    workspace: Res<Workspace>,
) {
    let some_dropped = q_dropped.iter().count() > 0;
    let single_click = i_mouse_button.just_released(MouseButton::Left)
        && workspace.drag != Drag::Dropping
        && !some_dropped;
    let hovered_slot = q_hovered_slot.iter().next();
    let hovered_node = q_hovered_node.iter().next();

    if single_click {
        if let Some(entity) = hovered_slot {
            // Select the one slot
            commands.entity(entity).insert(Selected);
            commands.entity(entity).insert(Active);
        } else if let Some((_, node_id)) = hovered_node {
            // Select the one node
            undo_command_manager.push(Box::new(ReplaceSelection(vec![node_id.0])));
            undo_command_manager.push(Box::new(MakeNodeActive(node_id.0)));
        } else {
            // Deselect everything.
            undo_command_manager.push(Box::new(DeselectAll));
            undo_command_manager.push(Box::new(MakeNothingActive));
        }
        undo_command_manager.push(Box::new(Checkpoint));
    } else if workspace.drag == Drag::Starting {
        if let Some(entity) = hovered_slot {
            // Drag on slot
            for entity in q_selected_slot.iter() {
                commands.entity(entity).remove::<Selected>();
            }
            commands.entity(entity).insert(Selected);
            tool_state
                .overwrite_replace(ToolState::Grab(GrabToolType::Slot))
                .unwrap();
        } else if let Some((_entity, node_id)) = hovered_node {
            // Drag on node
            let some_hovered_selected_node = q_hovered_selected_node.iter().count() > 0;
            if !some_hovered_selected_node {
                undo_command_manager.push(Box::new(DeselectAll));
                undo_command_manager.push(Box::new(SelectNode(node_id.0)));
            }
            tool_state
                .overwrite_replace(ToolState::Grab(GrabToolType::Node))
                .unwrap();
        } else {
            // Drag on empty space
            tool_state.overwrite_replace(ToolState::BoxSelect).unwrap();
        }
    }
}
