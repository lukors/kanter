pub mod edge;
pub mod node;

use std::fmt::Debug;

use crate::{
    shared::NodeIdComponent,
    undo::{prelude::*, undo_command_manager::Checkpoint},
    AmbiguitySet, CustomStage, GrabToolType, Slot, ToolState,
};
use bevy::prelude::*;

use self::node::{grab_node_setup, MoveNodeUndo};
use self::{
    edge::{
        drag_edge_update, dropped_edge_update, grab_tool_slot_setup, grabbed_edge_update,
        spawn_grabbed_edges,
    },
    node::{grab_node_cleanup, grab_node_update},
};

#[derive(Component, Default)]
pub(crate) struct Draggable;
#[derive(Component, Default)]
pub(crate) struct Dragged {
    start: Vec2,
}
#[derive(Component, Default)]
pub(crate) struct Dropped {
    start: Vec2,
    end: Vec2,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum DragDropStage {
    Setup,
    Node,
    Edge,
}

pub(crate) struct WorkspaceDragDropPlugin;

impl Plugin for WorkspaceDragDropPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(CustomStage::Update)
                .label(DragDropStage::Setup)
                .after(CustomStage::Setup)
                .with_system(dropped_update.system())
                .with_system(
                    grab_tool_slot_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        )
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(DragDropStage::Node)
                .after(DragDropStage::Setup)
                .with_system(
                    grab_node_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_node_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_node_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                ),
        )
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(DragDropStage::Edge)
                .after(DragDropStage::Node)
                .with_system(
                    spawn_grabbed_edges
                        .system()
                        .chain(grabbed_edge_update.system())
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    dropped_edge_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(drag_edge_update.system()),
        );
    }
}

/// Exit grab tool if mouse button is released.
fn grab_tool_update(
    mut commands: Commands,
    q_dragged: Query<(Entity, &Dragged, &GlobalTransform)>,
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
) {
    // Todo: Replace with specific solutions that create UndoCommands before exiting tool.
    if i_mouse_button.just_released(MouseButton::Left) {
        for (entity, dragged, gtransform) in q_dragged.iter() {
            commands.entity(entity).remove::<Dragged>();
            commands.entity(entity).insert(Dropped {
                start: dragged.start,
                end: gtransform.translation.truncate(),
            });
        }
        // tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

/// Drops all grabbed entities.
pub(crate) fn grab_tool_cleanup(
    mut commands: Commands,
    q_dragged: Query<(Entity, &Dragged, &GlobalTransform)>,
) {
    // Todo: This should be replaced with a specific solution for edges, and one for nodes.
    // They need different cleanup.

    for (entity, dragged, gtransform) in q_dragged.iter() {
        commands.entity(entity).remove::<Dragged>();
        commands.entity(entity).insert(Dropped {
            start: dragged.start,
            end: gtransform.translation.truncate(),
        });
    }
}

/// When an entity gets the `Dropped` component, this system returns it to its un-dragged state.
fn dropped_update(
    mut undo_command_manager: ResMut<UndoCommandManager>,
    mut commands: Commands,
    mut q_dropped: Query<
        (Entity, Option<&Slot>, Option<&NodeIdComponent>, &Dropped),
        Added<Dropped>,
    >,
) {
    // Todo: Should this be replaced with a specific solution for each type of draggable?
    let mut changed = false;

    for (entity, slot_id, node_id, transform) in q_dropped.iter_mut() {
        if slot_id.is_none() {
            commands.entity(entity).remove::<Parent>();

            if let (Some(node_id), dropped) = (node_id, transform) {
                undo_command_manager.push(Box::new(MoveNodeUndo {
                    node_id: node_id.0,
                    from: dropped.start,
                    to: dropped.end,
                }));
                changed = true;
            }
        }
        commands.entity(entity).remove::<Dropped>();
    }

    if changed {
        undo_command_manager.push(Box::new(Checkpoint));
    }
}
