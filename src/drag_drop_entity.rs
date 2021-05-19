/// Dragging and dropping nodes and edges.
use crate::{
    control_pressed, hoverable::box_contains_point, scan_code_input::ScanCodeInput,
    stretch_between, AmbiguitySet, Cursor, Edge, GrabToolType, Selected, Slot, Stage, ToolState,
};
use bevy::prelude::*;
use kanter_core::{node::Side, node_graph::NodeId, texture_processor::TextureProcessor};

#[derive(Default)]
pub(crate) struct Draggable;
#[derive(Default)]
pub(crate) struct Dragged;
#[derive(Default)]
pub(crate) struct Dropped;
struct SourceSlot(Slot);

struct GrabbedEdge {
    start: Vec2,
    slot: Slot,
}

pub(crate) struct WorkspaceDragDropPlugin;

impl Plugin for WorkspaceDragDropPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Update)
                .after(Stage::Input)
                // Node
                .with_system(
                    grab_tool_node_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    drag_node_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_cleanup
                        .system()
                        .with_run_criteria(State::on_exit(ToolState::Grab(GrabToolType::Node)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                // Slot
                .with_system(
                    grab_tool_slot_setup
                        .system()
                        .with_run_criteria(State::on_enter(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    spawn_grabbed_edges
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grab_tool_update
                        .system()
                        .with_run_criteria(State::on_update(ToolState::Grab(GrabToolType::Slot)))
                        .in_ambiguity_set(AmbiguitySet),
                )
                .with_system(
                    grabbed_edge_update
                        .system()
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
                ),
        )
        .add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Apply)
                .after(Stage::Update)
                .with_system(dropped_update.system())
                .with_system(drag_node_update.system())
                .with_system(update_edges.system()),
        );
    }
}

/// When an edge is dropped, this system updates the node graph based on where its dropped, and
/// removes the edges.
#[allow(clippy::too_many_arguments)]
fn dropped_edge_update(
    mut commands: Commands,
    mut tool_state: ResMut<State<ToolState>>,
    tex_pro: ResMut<TextureProcessor>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_slot: Query<(&GlobalTransform, &Sprite, &Slot)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
    q_grabbed_edge: Query<(Entity, &GrabbedEdge, Option<&SourceSlot>)>,
    mut q_edge: Query<&mut Visible, With<Edge>>,
) {
    if i_mouse_button.just_released(MouseButton::Left) {
        let cursor_t = q_cursor.iter().next().unwrap();

        'outer: for (_, grabbed_edge, source_slot) in q_grabbed_edge.iter() {
            for (slot_t, slot_sprite, slot) in q_slot.iter() {
                if box_contains_point(
                    slot_t.translation.truncate(),
                    slot_sprite.size,
                    cursor_t.translation.truncate(),
                ) {
                    if tex_pro
                        .connect_arbitrary(
                            slot.node_id,
                            slot.side,
                            slot.slot_id,
                            grabbed_edge.slot.node_id,
                            grabbed_edge.slot.side,
                            grabbed_edge.slot.slot_id,
                        )
                        .is_ok()
                    {
                        if let Some(source_slot) = source_slot {
                            if source_slot.0 != *slot {
                                if let Err(e) = tex_pro.disconnect_slot(
                                    source_slot.0.node_id,
                                    source_slot.0.side,
                                    source_slot.0.slot_id,
                                ) {
                                    error!(
                                        "Failed to disconnect slot: {}, {:?}, {}: {}",
                                        source_slot.0.node_id,
                                        source_slot.0.side,
                                        source_slot.0.slot_id,
                                        e
                                    );
                                }
                            }
                        }
                        continue 'outer;
                    } else {
                        trace!("Failed to connect nodes");
                        continue 'outer;
                    }
                }
            }
            if let Some(source_slot) = source_slot {
                if let Err(e) = tex_pro.disconnect_slot(
                    source_slot.0.node_id,
                    source_slot.0.side,
                    source_slot.0.slot_id,
                ) {
                    error!(
                        "Failed to disconnect slot: {}, {:?}, {}: {}",
                        source_slot.0.node_id, source_slot.0.side, source_slot.0.slot_id, e
                    );
                }
            }
        }

        for (edge_e, _, _) in q_grabbed_edge.iter() {
            commands.entity(edge_e).despawn_recursive();
        }

        for mut visible in q_edge.iter_mut() {
            visible.is_visible = true;
        }

        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

/// Updates the visual of all dragged slots.
fn grabbed_edge_update(
    mut q_edge: Query<(&mut Transform, &GrabbedEdge, &mut Sprite)>,
    q_cursor: Query<&GlobalTransform, With<Cursor>>,
) {
    if let Ok(cursor_t) = q_cursor.single() {
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
    q_selected_slots: Query<Entity, (With<Slot>, With<Selected>)>,
) {
    if q_selected_slots.iter().count() == 0 {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for entity in q_selected_slots.iter() {
        commands.entity(entity).insert(Dragged);
    }
}

/// Grab all selected nodes.
pub(crate) fn grab_tool_node_setup(
    mut tool_state: ResMut<State<ToolState>>,
    mut commands: Commands,
    q_selected_nodes: Query<Entity, (With<NodeId>, With<Selected>)>,
) {
    if q_selected_nodes.iter().count() == 0 {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for entity in q_selected_nodes.iter() {
        commands.entity(entity).insert(Dragged);
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
pub(crate) fn grab_tool_cleanup(mut commands: Commands, q_dragged: Query<Entity, With<Dragged>>) {
    for entity in q_dragged.iter() {
        commands.entity(entity).remove::<Dragged>();
        commands.entity(entity).insert(Dropped);
    }
}

/// Updates all dragged nodes.
fn drag_node_update(
    mut commands: Commands,
    mut q_dragged_node: Query<
        (Entity, &mut Transform, &GlobalTransform),
        (Added<Dragged>, With<NodeId>, Without<Slot>),
    >,
    q_cursor: Query<(Entity, &GlobalTransform), With<Cursor>>,
) {
    if let Ok((cursor_e, cursor_transform)) = q_cursor.single() {
        for (entity, mut transform, global_transform) in q_dragged_node.iter_mut() {
            let global_pos = global_transform.translation - cursor_transform.translation;
            transform.translation.x = global_pos.x;
            transform.translation.y = global_pos.y;

            commands.entity(cursor_e).push_children(&[entity]);
        }
    }
}

/// When a slot is grabbed this system creates its edge entity.
fn spawn_grabbed_edges(
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    q_dragged_slot: Query<(&GlobalTransform, &Slot), Added<Dragged>>,
    q_slot: Query<(&GlobalTransform, &Slot)>,
    mut q_edge: Query<(&mut Visible, &Edge)>,
    scan_code_input: Res<ScanCodeInput>,
) {
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
                                .spawn_bundle(SpriteBundle {
                                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                                    sprite: Sprite::new(Vec2::new(5., 5.)),
                                    ..Default::default()
                                })
                                .insert(GrabbedEdge {
                                    start: input_slot_gtransform.translation.truncate(),
                                    slot: input_slot.clone(),
                                })
                                .insert(SourceSlot(dragged_slot.clone()));
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
                                .spawn_bundle(SpriteBundle {
                                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                                    sprite: Sprite::new(Vec2::new(5., 5.)),
                                    ..Default::default()
                                })
                                .insert(GrabbedEdge {
                                    start: output_slot_gtransform.translation.truncate(),
                                    slot: output_slot.clone(),
                                })
                                .insert(SourceSlot(dragged_slot.clone()));
                        }
                    }
                }
            }
        } else {
            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.add(Color::rgb(0., 0., 0.).into()),
                    sprite: Sprite::new(Vec2::new(5., 5.)),
                    ..Default::default()
                })
                .insert(GrabbedEdge {
                    start: dragged_slot_gtransform.translation.truncate(),
                    slot: dragged_slot.clone(),
                });
        }
    }
}

/// When an entity gets the `Dropped` component, this system returns it to its un-dragged state.
fn dropped_update(
    mut commands: Commands,
    mut q_dropped: Query<(Entity, Option<&Slot>), Added<Dropped>>,
) {
    for (entity, slot_id) in q_dropped.iter_mut() {
        if slot_id.is_none() {
            commands.entity(entity).remove::<Parent>();
        }
        commands.entity(entity).remove::<Dropped>();
    }
}

fn update_edges(
    q_node: Query<&NodeId, With<Dragged>>,
    q_slot: Query<(&Slot, &GlobalTransform)>,
    mut q_edge: Query<(&mut Sprite, &mut Transform, &Edge)>,
) {
    for node_id in q_node.iter() {
        for (mut sprite, mut transform, edge) in q_edge.iter_mut().filter(|(_, _, edge)| {
            edge.input_slot.node_id == *node_id || edge.output_slot.node_id == *node_id
        }) {
            let (mut start, mut end) = (Vec2::ZERO, Vec2::ZERO);

            for (slot, slot_t) in q_slot.iter() {
                if slot.node_id == edge.output_slot.node_id
                    && slot.slot_id == edge.output_slot.slot_id
                    && slot.side == edge.output_slot.side
                {
                    start = slot_t.translation.truncate();
                } else if slot.node_id == edge.input_slot.node_id
                    && slot.slot_id == edge.input_slot.slot_id
                    && slot.side == edge.input_slot.side
                {
                    end = slot_t.translation.truncate();
                }
            }

            stretch_between(&mut sprite, &mut transform, start, end);
        }
    }
}
