/// All workspace mouse interaction.
use crate::{
    shared::NodeIdComponent,
    undo::{prelude::*, UndoCommand, UndoCommandType},
    AmbiguitySet, CustomStage, Drag, Dropped, GrabToolType, Hovered, Slot, ToolState, Workspace,
};
use bevy::prelude::*;
use kanter_core::node_graph::NodeId;

#[derive(Component, Default)]
pub(crate) struct Active;

#[derive(Component, Default)]
pub(crate) struct Selected;

fn select_node(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), Without<Selected>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).insert(Selected);
    } else {
        warn!("failed to select a node");
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
        warn!("failed to deselect a node");
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

fn make_node_active(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), Without<Active>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).insert(Active);
    } else {
        warn!("failed to make a node active");
    }
}

fn make_node_not_active(world: &mut World, node_id: NodeId) {
    let mut q_node_id = world.query_filtered::<(Entity, &NodeIdComponent), With<Active>>();

    if let Some((entity, _)) = q_node_id
        .iter(world)
        .find(|(_, node_id_component)| node_id_component.0 == node_id)
    {
        world.entity_mut(entity).remove::<Active>();
    } else {
        warn!("failed to make a node not active");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNodeActive(pub NodeId);
impl UndoCommand for MakeNodeActive {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }
    
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_active_node_id = world.query_filtered::<&NodeIdComponent, With<Active>>();
        assert!(q_active_node_id.iter(world).count() < 2, "there is more than one active node");
        
        if let Some(active_node_id) = q_active_node_id.iter(world).next() {
            if active_node_id.0 != self.0 {
                undo_command_manager.push_front(Box::new(MakeNodeNotActiveOnly(active_node_id.0)));
                undo_command_manager.push_front(Box::new(MakeNodeActiveOnly(self.0)));
            }
        } else {
            undo_command_manager.push_front(Box::new(MakeNodeActiveOnly(self.0)));
        }
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNodeActiveOnly(pub NodeId);
impl UndoCommand for MakeNodeActiveOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_active(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_not_active(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNodeNotActive(pub NodeId);
impl UndoCommand for MakeNodeNotActive {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }
    
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_active_node_id = world.query_filtered::<&NodeIdComponent, With<Active>>();
        assert!(q_active_node_id.iter(world).count() < 2, "there is more than one active node");
        
        if let Some(active_node_id) = q_active_node_id.iter(world).next() {
            if active_node_id.0 == self.0 {
                undo_command_manager.push_front(Box::new(MakeNodeNotActiveOnly(self.0)));
            } else {
                warn!("tried making a not active node not active");
            }
        } else {
            warn!("could not find an active node to make not active");
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is never put on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNodeNotActiveOnly(pub NodeId);
impl UndoCommand for MakeNodeNotActiveOnly {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_not_active(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        make_node_active(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MakeNothingActive;
impl UndoCommand for MakeNothingActive {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }
    
    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_active_node_id = world.query_filtered::<&NodeIdComponent, With<Active>>();
        assert!(q_active_node_id.iter(world).count() < 2, "there is more than one active node");
        
        if let Some(node_id) = q_active_node_id.iter(world).next() {
            undo_command_manager.push_front(Box::new(MakeNodeNotActive(node_id.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this undo command is never put on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeselectNode(pub NodeId);
impl UndoCommand for DeselectNode {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        deselect_node(world, self.0);
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        select_node(world, self.0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DeselectAll;
impl UndoCommand for DeselectAll {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_selected = world.query_filtered::<&NodeIdComponent, With<Selected>>();

        for node_id in q_selected.iter(world) {
            undo_command_manager
                .commands
                .push_front(Box::new(DeselectNode(node_id.0)));
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("command is never put on undo stack");
    }
}

#[derive(Debug)]
pub struct ReplaceSelection(pub Vec<NodeId>);
impl UndoCommand for ReplaceSelection {
    fn command_type(&self) -> UndoCommandType {
        UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, undo_command_manager: &mut UndoCommandManager) {
        let mut q_node_id = world.query::<(&NodeIdComponent, Option<&Selected>)>();

        for (node_id, selected) in q_node_id.iter(world) {
            let in_new_selection = self.0.contains(&node_id.0);

            if selected.is_none() && in_new_selection {
                undo_command_manager
                    .commands
                    .push_front(Box::new(SelectNode(node_id.0)));
            } else if selected.is_some() && !in_new_selection {
                undo_command_manager
                    .commands
                    .push_front(Box::new(DeselectNode(node_id.0)));
            }
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
            // for entity in q_active.iter() {
                // undo_command_manager.push(Box::new(MakeNodeNotActive(node_id.0)));
            // }
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
