use anyhow::{anyhow, Result};
use std::fmt::Debug;

/// Dragging and dropping nodes and edges.
use crate::{
    control_pressed,
    hoverable::box_contains_point,
    scan_code_input::ScanCodeInput,
    shared::NodeIdComponent,
    stretch_between,
    // thumbnail::ThumbnailState,
    undo::{
        edge::{AddEdge, RemoveGuiEdge},
        prelude::*,
        undo_command_manager::Checkpoint,
    },
    AmbiguitySet,
    Cursor,
    CustomStage,
    Edge as GuiEdge,
    GrabToolType,
    Selected,
    Slot,
    ToolState,
};
use bevy::prelude::*;
use kanter_core::{edge::Edge, node::Side, node_graph::NodeId};

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

#[derive(Component, Copy, Clone, Debug)]
struct SourceSlot(Slot);

#[derive(Component)]
struct GrabbedEdge {
    start: Vec2,
    slot: Slot,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum DragDropStage {
    Setup,
    Node,
    GuiEdge,
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
                    grab_tool_node_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(drag_node_update.system()),
        )
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(DragDropStage::GuiEdge)
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
                .with_system(drag_edge_update.system()), // )
                                                         // .add_system_set_to_stage(
                                                         //     CoreStage::Update,
                                                         //     SystemSet::new()
                                                         //         .after(DragDropStage::GuiEdge)
        );
    }
}

/// When an edge is dropped, this system updates the node graph based on where its dropped, and
/// removes the edges.
#[allow(clippy::too_many_arguments)]
fn dropped_edge_update(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_slot: Query<(&GlobalTransform, &Sprite, &Slot)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
    q_grabbed_edge: Query<(Entity, &GrabbedEdge, Option<&SourceSlot>)>,
    mut q_edge_visible: Query<&mut Visibility, With<GuiEdge>>,
    q_edge: Query<&GuiEdge>,
    // mut q_thumbnail_state: Query<(&NodeId, &mut ThumbnailState), Without<GrabbedEdge>>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    if i_mouse_button.just_released(MouseButton::Left) {
        let cursor_t = q_cursor.iter().next().unwrap();
        let mut new_edges = Vec::new();

        'outer: for (_, grabbed_edge, source_slot) in q_grabbed_edge.iter() {
            for (slot_t, slot_sprite, slot) in q_slot.iter() {
                if let Some(size) = slot_sprite.custom_size {
                    if box_contains_point(
                        slot_t.translation.truncate(),
                        size,
                        cursor_t.translation.truncate(),
                    ) {
                        if let Some(source_slot) = source_slot {
                            if source_slot.0 != *slot {
                                if let Ok(add_edge) = connect_arbitrary(*slot, grabbed_edge.slot) {
                                    new_edges.push(Box::new(add_edge));
                                }
                                for edge in q_edge.iter() {
                                    if (edge.input_slot == source_slot.0
                                        && edge.output_slot == grabbed_edge.slot)
                                        || (edge.output_slot == source_slot.0
                                            && edge.input_slot == grabbed_edge.slot)
                                    {
                                        undo_command_manager.push(Box::new(RemoveGuiEdge(*edge)));
                                    }
                                }
                            }
                        } else if let Ok(add_edge) = connect_arbitrary(*slot, grabbed_edge.slot) {
                            new_edges.push(Box::new(add_edge));
                        }

                        continue 'outer;
                    }
                }
            }
            if let Some(source_slot) = source_slot {
                for edge in q_edge.iter() {
                    if (edge.input_slot == source_slot.0 && edge.output_slot == grabbed_edge.slot)
                        || (edge.output_slot == source_slot.0
                            && edge.input_slot == grabbed_edge.slot)
                    {
                        undo_command_manager.push(Box::new(RemoveGuiEdge(*edge)));
                    }
                }
            }
        }

        for new_edge in new_edges {
            undo_command_manager.push(new_edge);
        }
        undo_command_manager.push(Box::new(Checkpoint));

        for (edge_e, _, _) in q_grabbed_edge.iter() {
            commands.entity(edge_e).despawn_recursive();
        }

        for mut visible in q_edge_visible.iter_mut() {
            visible.is_visible = true;
        }

        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

fn connect_arbitrary(slot_a: Slot, slot_b: Slot) -> Result<AddEdge> {
    if let Ok(edge) = Edge::from_arbitrary(
        slot_a.node_id,
        slot_a.side,
        slot_a.slot_id,
        slot_b.node_id,
        slot_b.side,
        slot_b.slot_id,
    ) {
        Ok(AddEdge(edge))
    } else {
        Err(anyhow!("could not connect slot"))
    }
}

/// Updates the visual of all dragged slots.
fn grabbed_edge_update(
    mut q_edge: Query<(&mut Transform, &GrabbedEdge, &mut Sprite)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
) {
    if let Ok(cursor_t) = q_cursor.get_single() {
        for (mut edge_t, edge, mut sprite) in q_edge.iter_mut() {
            stretch_between(
                &mut sprite,
                &mut edge_t,
                edge.start,
                cursor_t.translation.truncate(),
            );
        }
    }
}

/// Grab all selected slots.
fn grab_tool_slot_setup(
    mut tool_state: ResMut<State<ToolState>>,
    mut commands: Commands,
    q_selected_slots: Query<(Entity, &GlobalTransform), (With<Slot>, With<Selected>)>,
) {
    if q_selected_slots.iter().count() == 0 {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for (entity, gtransform) in q_selected_slots.iter() {
        commands.entity(entity).insert(Dragged {
            start: gtransform.translation.truncate(),
        });
    }
}

/// Grab all selected nodes.
pub(crate) fn grab_tool_node_setup(
    mut tool_state: ResMut<State<ToolState>>,
    mut commands: Commands,
    q_selected_nodes: Query<(Entity, &GlobalTransform), (With<NodeIdComponent>, With<Selected>)>,
) {
    if q_selected_nodes.iter().count() == 0 {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for (entity, gtransform) in q_selected_nodes.iter() {
        commands.entity(entity).insert(Dragged {
            start: gtransform.translation.truncate(),
        });
    }
}

/// Exit grab tool if mouse button is released.
fn grab_tool_update(
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
) {
    if i_mouse_button.just_released(MouseButton::Left) {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

/// Drops all grabbed entities.
pub(crate) fn grab_tool_cleanup(
    mut commands: Commands,
    q_dragged: Query<(Entity, &Dragged, &GlobalTransform)>,
) {
    for (entity, dragged, gtransform) in q_dragged.iter() {
        commands.entity(entity).remove::<Dragged>();
        commands.entity(entity).insert(Dropped {
            start: dragged.start,
            end: gtransform.translation.truncate(),
        });
    }
}

/// Updates all dragged nodes.
fn drag_node_update(
    mut commands: Commands,
    mut q_dragged_node: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (Added<Dragged>, With<NodeIdComponent>, Without<Slot>),
    >,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
) {
    if let Ok((cursor_e, cursor_transform)) = q_cursor.get_single() {
        for (entity, mut transform, global_transform) in q_dragged_node.iter_mut() {
            commands.entity(cursor_e).push_children(&[entity]);

            let global_pos = global_transform.translation - cursor_transform.translation;
            transform.translation.x = global_pos.x;
            transform.translation.y = global_pos.y;
        }
    }
}

/// When a slot is grabbed this system creates its edge entity.
fn spawn_grabbed_edges(
    mut commands: Commands,
    q_dragged_slot: Query<(&GlobalTransform, &Slot), Added<Dragged>>,
    q_slot: Query<(&GlobalTransform, &Slot)>,
    mut q_edge: Query<(&mut Visibility, &GuiEdge)>,
    scan_code_input: Res<ScanCodeInput>,
) {
    let line_sprite_bundle = SpriteBundle {
        sprite: Sprite {
            color: Color::BLACK,
            custom_size: Some(Vec2::new(5.0, 5.0)),
            ..Default::default()
        },
        ..Default::default()
    };

    for (dragged_slot_gtransform, dragged_slot) in q_dragged_slot.iter() {
        if control_pressed(&scan_code_input) {
            match dragged_slot.side {
                Side::Output => {
                    for (mut edge_visible, edge) in q_edge
                        .iter_mut()
                        .filter(|(_, edge)| edge.output_slot == *dragged_slot)
                    {
                        edge_visible.is_visible = false;

                        if let Some((input_slot_gtransform, input_slot)) =
                            q_slot.iter().find(|(_, slot)| {
                                slot.node_id == edge.input_slot.node_id
                                    && slot.slot_id == edge.input_slot.slot_id
                                    && slot.side == Side::Input
                            })
                        {
                            commands
                                .spawn_bundle(line_sprite_bundle.clone())
                                .insert(GrabbedEdge {
                                    start: input_slot_gtransform.translation.truncate(),
                                    slot: *input_slot,
                                })
                                .insert(SourceSlot(*dragged_slot));
                        }
                    }
                }
                Side::Input => {
                    if let Some((mut edge_visible, edge)) = q_edge
                        .iter_mut()
                        .find(|(_, edge)| edge.input_slot == *dragged_slot)
                    {
                        edge_visible.is_visible = false;

                        if let Some((output_slot_gtransform, output_slot)) =
                            q_slot.iter().find(|(_, slot)| {
                                slot.node_id == edge.output_slot.node_id
                                    && slot.slot_id == edge.output_slot.slot_id
                                    && slot.side == Side::Output
                            })
                        {
                            commands
                                .spawn_bundle(line_sprite_bundle.clone())
                                .insert(GrabbedEdge {
                                    start: output_slot_gtransform.translation.truncate(),
                                    slot: *output_slot,
                                })
                                .insert(SourceSlot(*dragged_slot));
                        }
                    }
                }
            }
        } else {
            commands
                .spawn_bundle(line_sprite_bundle.clone())
                .insert(GrabbedEdge {
                    start: dragged_slot_gtransform.translation.truncate(),
                    slot: *dragged_slot,
                });
        }
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

fn drag_edge_update(
    q_node: Query<(&NodeIdComponent, &Transform), With<Dragged>>,
    q_slot: Query<(&Slot, &Transform)>,
    mut q_edge: Query<
        (&mut Sprite, &mut Transform, &mut GuiEdge),
        (Without<NodeIdComponent>, Without<Slot>, Without<Cursor>),
    >,
    q_cursor: Query<&Transform, With<Cursor>>,
) {
    let cursor_t = q_cursor.iter().next().unwrap().translation;

    for (node_id, node_t) in q_node.iter() {
        for (mut sprite, mut edge_t, mut edge) in q_edge.iter_mut().filter(|(_, _, edge)| {
            edge.input_slot.node_id == node_id.0 || edge.output_slot.node_id == node_id.0
        }) {
            for (slot, slot_t) in q_slot.iter().filter(|(slot, _)| slot.node_id == node_id.0) {
                if slot.node_id == edge.output_slot.node_id
                    && slot.slot_id == edge.output_slot.slot_id
                    && slot.side == edge.output_slot.side
                {
                    edge.start = (cursor_t + node_t.translation + slot_t.translation).truncate();
                } else if slot.node_id == edge.input_slot.node_id
                    && slot.slot_id == edge.input_slot.slot_id
                    && slot.side == edge.input_slot.side
                {
                    edge.end = (cursor_t + node_t.translation + slot_t.translation).truncate();
                }
            }

            stretch_between(&mut sprite, &mut edge_t, edge.start, edge.end);
        }
    }
}

#[derive(Clone, Debug)]
pub struct MoveNodeUndo {
    node_id: NodeId,
    from: Vec2,
    to: Vec2,
}

impl UndoCommand for MoveNodeUndo {
    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query::<(&NodeIdComponent, &mut Transform)>();
        if let Some((_, mut transform)) = query
            .iter_mut(world)
            .find(|(node_id, _)| node_id.0 == self.node_id)
        {
            transform.translation.x = self.to.x;
            transform.translation.y = self.to.y;
            update_node_gui_edges(world, self.node_id);
        }
    }

    fn backward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query::<(&NodeIdComponent, &mut Transform)>();
        if let Some((_, mut transform)) = query
            .iter_mut(world)
            .find(|(node_id, _)| node_id.0 == self.node_id)
        {
            transform.translation.x = self.from.x;
            transform.translation.y = self.from.y;
            update_node_gui_edges(world, self.node_id);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct DragToolUndo;
impl UndoCommand for DragToolUndo {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut tool_state = world.get_resource_mut::<State<ToolState>>().unwrap();
        let _ = tool_state.overwrite_replace(ToolState::Grab(GrabToolType::Node));
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SelectNew;
impl UndoCommand for SelectNew {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query_filtered::<Entity, (With<Draggable>, Added<NodeIdComponent>)>();
        for entity in query.iter(world).collect::<Vec<Entity>>() {
            world.entity_mut(entity).insert(Selected);
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

/// The sneaky variant is not saved on the undo stack. Can probably be replaced with a command that
/// removes the most recent command from the undo stack.
#[derive(Copy, Clone, Debug)]
pub struct SelectedToCursorSneaky;
impl UndoCommand for SelectedToCursorSneaky {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query =
            world.query_filtered::<&mut Transform, (With<Selected>, With<NodeIdComponent>)>();
        let cursor = *world
            .query_filtered::<&GlobalTransform, With<Cursor>>()
            .iter(world)
            .next()
            .unwrap();

        for mut transform in query.iter_mut(world) {
            transform.translation.x = cursor.translation.x;
            transform.translation.y = cursor.translation.y;
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

/// The sneaky variant is not saved on the undo stack. Can probably be replaced with a command that
/// removes the most recent command from the undo stack.
#[derive(Copy, Clone, Debug)]
pub struct DeselectSneaky;
impl UndoCommand for DeselectSneaky {
    fn command_type(&self) -> crate::undo::UndoCommandType {
        crate::undo::UndoCommandType::Custom
    }

    fn forward(&self, world: &mut World, _: &mut UndoCommandManager) {
        let mut query = world.query_filtered::<Entity, With<Selected>>();

        for entity in query.iter(world).collect::<Vec<Entity>>() {
            world.entity_mut(entity).remove::<Selected>();
        }
    }

    fn backward(&self, _: &mut World, _: &mut UndoCommandManager) {
        unreachable!("this command is not saved on the undo stack");
    }
}

fn update_node_gui_edges(world: &mut World, node_id: NodeId) {
    let node_transform = *world
        .query::<(&NodeIdComponent, &Transform)>()
        .iter(world)
        .find(|(node_id_iter, _)| node_id_iter.0 == node_id)
        .map(|(_, transform)| transform)
        .unwrap();
    let slots = world
        .query::<(&Slot, &Transform)>()
        .iter(world)
        .filter(|(slot, _)| slot.node_id == node_id)
        .map(|(slot, transform)| (*slot, *transform))
        .collect::<Vec<(Slot, Transform)>>();

    let mut q_edge = world.query::<(&mut Sprite, &mut Transform, &mut GuiEdge)>();

    for (mut sprite, mut edge_t, mut edge) in q_edge.iter_mut(world).filter(|(_, _, edge)| {
        edge.input_slot.node_id == node_id || edge.output_slot.node_id == node_id
    }) {
        for (slot, slot_t) in slots.iter() {
            if slot.node_id == edge.output_slot.node_id
                && slot.slot_id == edge.output_slot.slot_id
                && slot.side == edge.output_slot.side
            {
                edge.start = (node_transform.translation + slot_t.translation).truncate();
            } else if slot.node_id == edge.input_slot.node_id
                && slot.slot_id == edge.input_slot.slot_id
                && slot.side == edge.input_slot.side
            {
                edge.end = (node_transform.translation + slot_t.translation).truncate();
            }
        }

        stretch_between(&mut sprite, &mut edge_t, edge.start, edge.end);
    }
}
