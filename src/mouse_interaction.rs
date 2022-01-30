/// All workspace mouse interaction.
use crate::{
    shared::NodeIdComponent,
    undo::{prelude::*, UndoCommand},
    AmbiguitySet, CustomStage, Drag, Dropped, GrabToolType, Hovered, Slot, ToolState, Workspace,
};
use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

#[derive(Component, Default)]
pub(crate) struct Active;

#[derive(Component, Default)]
pub(crate) struct Selected;

fn select_node(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query::<(Entity, &NodeIdComponent)>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).insert(Selected);
    } else {
        warn!("tried and failed to select a node");
    }
}

fn deselect_node(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), With<Selected>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).remove::<Selected>();
    } else {
        warn!("tried and failed to deselect a node");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SelectNode(pub NodeId);
impl UndoCommand for SelectNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        select_node(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        deselect_node(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
struct DeselectNode(NodeId);
impl UndoCommand for DeselectNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        deselect_node(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        select_node(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
struct DeselectAll;
impl UndoCommand for DeselectAll {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }
    
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_selected = world.query_filtered::<&NodeIdComponent, With<Selected>>();

        for node_id in q_selected.iter(world) {
            undo_command_manager.commands.push_front(Box::new(DeselectNode(node_id.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("command is never put on undo stack");
    }
}

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
    q_hovered_node: Query<Entity, (With<NodeIdComponent>, With<Hovered>)>,
    q_selected_node: Query<Entity, (With<NodeIdComponent>, With<Selected>)>,
    q_hovered_selected_node: Query<Entity, (With<NodeIdComponent>, With<Selected>, With<Hovered>)>,
    q_hovered_slot: Query<Entity, (With<Slot>, With<Hovered>)>,
    q_selected_slot: Query<Entity, (With<Slot>, With<Selected>)>,
    q_selected: Query<&NodeIdComponent, With<Selected>>,
    q_active: Query<Entity, With<Active>>,
    q_dropped: Query<&Dropped>,
    workspace: Res<Workspace>,
) {
    let some_dropped = q_dropped.iter().count() > 0;
    let single_click = i_mouse_button.just_released(MouseButton::Left)
        && workspace.drag != Drag::Dropping
        && !some_dropped;

    if single_click {
        // Deselect everything.
        undo_command_manager.push(Box::new(DeselectAll));
        for entity in q_active.iter() {
            commands.entity(entity).remove::<Active>();
        }

        undo_command_manager.push(Box::new(Checkpoint));
    }

    if let Some(entity) = q_hovered_slot.iter().next() {
        // Slot
        if single_click {
            // Select the one slot
            commands.entity(entity).insert(Selected);
            commands.entity(entity).insert(Active);
        } else if workspace.drag == Drag::Starting {
            // Drag on slot
            for entity in q_selected_slot.iter() {
                commands.entity(entity).remove::<Selected>();
            }
            commands.entity(entity).insert(Selected);
            tool_state
                .overwrite_replace(ToolState::Grab(GrabToolType::Slot))
                .unwrap();
        }
    } else if let Some(entity) = q_hovered_node.iter().next() {
        // Node
        if single_click {
            // Select the one node
            commands.entity(entity).insert(Selected);
            commands.entity(entity).insert(Active);
        } else if workspace.drag == Drag::Starting {
            // Drag on node
            let some_hovered_selected_node = q_hovered_selected_node.iter().count() > 0;
            if !some_hovered_selected_node {
                for entity in q_selected_node.iter() {
                    commands.entity(entity).remove::<Selected>();
                }

                commands.entity(entity).insert(Selected);
            }
            tool_state
                .overwrite_replace(ToolState::Grab(GrabToolType::Node))
                .unwrap();
        }
    } else if workspace.drag == Drag::Starting {
        // Drag on empty space
        tool_state.overwrite_replace(ToolState::BoxSelect).unwrap();
    }
}
