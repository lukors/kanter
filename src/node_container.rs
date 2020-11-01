use crate::{edge::Edge, menu_property::MenuProperty, node::Node, shared::*, slot::Slot};
use kanter_core::{
    node::{Node as CoreNode, MixType, NodeType, Side},
    node_graph::{Edge as CoreEdge, NodeGraph, NodeId, SlotId},
};
use orbtk::{prelude::*, shell::event::MouseButton};
use serde::{Deserialize, Serialize};
use std::fs::File;

const DRAG_THRESHOLD: f64 = 5.;

#[derive(Default, Serialize, Deserialize)]
struct NodeGraphSpatial {
    locations: Vec<Location>,
    node_graph: NodeGraph,
}

#[derive(Clone, Serialize, Deserialize)]
struct Location {
    node_id: NodeId,
    point: (f64, f64),
}

type List = Vec<String>;
widget!(NodeContainer<NodeContainerState> {
    action: OptionAction,
    action_main: OptionActionMain,
    add_node: OptionNodeType,
    menu_property_list: List
});

impl Template for NodeContainer {
    fn template(self, _id: Entity, _ctx: &mut BuildContext) -> Self {
        self.name("NodeContainer")
    }
}

const DRAG_OFFSET_DEFAULT: Point = Point {
    x: NODE_WIDTH / 2.,
    y: NODE_WIDTH / 2.,
};

#[derive(Default, AsAny)]
struct NodeContainerState {
    node_graph_spatial: NodeGraphSpatial,
    dragged_edges: (Vec<Entity>, WidgetSide),
    mouse_position: Point,
    drag_offset: Point,
    dragging: bool,
    dragged_entity: OptionDragDropEntity,
    dropped_on_entity: OptionDragDropEntity,
    selected_entity: OptionDragDropEntity,
    menu_property: Entity,
    menu_property_node: Option<Entity>,
    menu_property_list: Vec<Entity>,
}

impl State for NodeContainerState {
    fn init(&mut self, _: &mut Registry, ctx: &mut Context<'_>) {
        self.drag_offset = DRAG_OFFSET_DEFAULT;
        self.init_menu_property(ctx);
    }

    fn update(&mut self, _: &mut Registry, ctx: &mut Context<'_>) {
        self.sync_properties(ctx);

        self.handle_action(ctx);
        // self.handle_add_node(ctx);
        self.handle_dragged_entity(ctx);
        self.handle_dropped_entity(ctx);

        self.reset_mouse_action(ctx);

        self.handle_action_main(ctx);
    }

    fn messages(
        &mut self,
        mut messages: MessageReader,
        _registry: &mut Registry,
        ctx: &mut Context,
    ) {
        for message in messages.read::<Message>() {
            match message {
                Message::AddNode(node_type) => {
                    self.add_node(ctx, node_type)
                }
            }
        }
    }
}

impl NodeContainerState {
    fn init_menu_property(&mut self, ctx: &mut Context) {
        let self_entity = ctx.widget().entity();
        let bc = &mut ctx.build_context();

        let menu_property = MenuProperty::new().build(bc);
        self.menu_property = menu_property;

        bc.append_child(self_entity, menu_property);
    }

    fn get_clicked_child(&self, ctx: &mut Context, mouse: Mouse) -> Option<Entity> {
        for child_entity in child_entities(ctx).iter().rev() {
            if ctx
                .get_widget(*child_entity)
                .get::<Rectangle>("bounds")
                .contains((mouse.position.x(), mouse.position.y()))
                && Self::is_clickable(ctx, *child_entity)
            {
                return Some(*child_entity);
            }
        }
        None
    }

    fn sync_properties(&mut self, ctx: &mut Context) {
        let menu_property_node = if let Some(menu_property_node) = self.menu_property_node {
            menu_property_node
        } else {
            return
        };

        if !Self::entity_type(ctx, menu_property_node, WidgetType::Node) {
            return
        }

        let node_type = self.node_type_of_entity(ctx, menu_property_node).clone();
        let node_id = NodeId(*ctx.get_widget(menu_property_node).get::<u32>("node_id"));

        match node_type {
            NodeType::Mix(mix_type) => {
                let mix_type_index = *ctx.get_widget(self.menu_property_list[0]).get::<i32>("selected_index");
                let mix_type_menu = MixType::from_index(mix_type_index as usize).unwrap();

                if mix_type != mix_type_menu {
                    self.node_graph_spatial.node_graph.set_mix_type(node_id, mix_type_menu)
                    .expect("Crash when setting node type");
                }
            }
            NodeType::Image(path) => {
                let property_widget = ctx.get_widget(self.menu_property_list[0]);
                let path_menu = property_widget.get::<String>("text");

                if path_menu.to_string() != *path {
                    self.node_graph_spatial.node_graph.set_image_node_path(node_id, path_menu.to_string()).unwrap();
                }
            }
            NodeType::OutputGray => (),
            _ => todo!()
        }
    }

    fn handle_action(&mut self, ctx: &mut Context) {
        if let Some(action) = *ctx.widget().get::<OptionAction>("action") {
            match action {
                Action::Press(mouse) => {
                    let option_clicked_entity = self.get_clicked_child(ctx, mouse);

                    match mouse.button {
                        MouseButton::Left => {
                            if let Some(clicked_entity) = option_clicked_entity {
                                self.dragged_entity = Some(DragDropEntity {
                                    widget_type: *ctx
                                        .get_widget(clicked_entity)
                                        .get::<WidgetType>("widget_type"),
                                    entity: clicked_entity,
                                });

                                let dragged_entity_pos = {
                                    let widget = ctx.get_widget(clicked_entity);
                                    let margin = widget.get::<Thickness>("margin");

                                    Point {
                                        x: margin.left,
                                        y: margin.top,
                                    }
                                };

                                self.drag_offset =
                                    Point::new(mouse.position.x(), mouse.position.y()) - dragged_entity_pos;
                            }
                        }
                        MouseButton::Right => {
                            if let Some(clicked_entity) = option_clicked_entity {
                                if Self::entity_type(ctx, clicked_entity, WidgetType::Node) {
                                    self.open_menu_property(ctx, clicked_entity);
                                } else {
                                    self.close_menu_property(ctx);
                                }
                            } else {
                                self.close_menu_property(ctx);
                            }
                        }
                        _ => {}
                    };
                    self.mouse_position = Point {
                        x: mouse.position.x(),
                        y: mouse.position.y(),
                    };
                }
                Action::Release(mouse) => {
                    let widget_type = WidgetType::Slot;

                    for slot_entity in Self::children_type(ctx, widget_type) {
                        if ctx
                            .get_widget(slot_entity)
                            .get::<Rectangle>("bounds")
                            .contains((mouse.position.x(), mouse.position.y()))
                        {
                            self.dropped_on_entity = Some(DragDropEntity {
                                widget_type,
                                entity: slot_entity,
                            });
                        }
                    }

                    self.mouse_position = Point {
                        x: mouse.position.x(),
                        y: mouse.position.y(),
                    };
                }
                Action::Move(p) => self.mouse_position = p,
                Action::Delete => {
                    if let Some(selected_entity) = self.selected_entity {
                        self.delete_node(ctx, selected_entity.entity);
                        self.selected_entity = None;
                    }
                }
            }
        }
    }

    fn close_menu_property(&mut self, ctx: &mut Context) {
        self.menu_property_node = None;
        self.menu_property_list.clear();
        ctx.clear_children_of(self.menu_property);
    }

    fn node_type_of_entity(&self, ctx: &mut Context, node_entity: Entity) -> &NodeType {
        let node_id = NodeId(*ctx.get_widget(node_entity).get::<u32>("node_id"));
        &self.node_graph_spatial.node_graph.node_with_id(node_id).unwrap().node_type
    }

    fn open_menu_property(&mut self, ctx: &mut Context, node_entity: Entity) {
        ctx.clear_children_of(self.menu_property);
        self.menu_property_list.clear();
        ctx.get_widget(self.menu_property).get_mut::<Rectangle>("bounds").set_height(100.);

        let node_type = self.node_type_of_entity(ctx, node_entity);

        let bc = &mut ctx.build_context();
        let properties: Vec<Entity> = match *node_type {
            NodeType::Mix(mix_type) => {
                let mix_types = vec!["Add".to_string(), "Subtract".to_string(), "Multiply".to_string(), "Divide".to_string()];

                let mix_type_cb = MenuProperty::combo_box(mix_types, mix_type.index() as i32).build(bc);

                vec![mix_type_cb]
            }
            NodeType::Image(ref path) => {
                let path = if path.is_empty() {
                    "data/image_2.png".to_string()
                } else {
                    path.to_owned()
                };

                let path_box = MenuProperty::text_box(path).build(bc);

                vec![path_box]
            }
            NodeType::OutputGray => Vec::new(),
            _ => todo!()
        };

        let property_stack = Stack::new().build(bc);
        for property in &properties {
            bc.append_child(property_stack, *property);
        }

        self.menu_property_list = properties;

        self.menu_property_node = Some(node_entity);

        let container = Container::new()
            .background("#ff0000")
            .build(bc);

        bc.append_child(container, property_stack);
        bc.append_child(self.menu_property, container);

    }

    fn handle_dragged_entity(&mut self, ctx: &mut Context) {
        let dragged_entity = match self.dragged_entity {
            Some(drag_drop_entity) => drag_drop_entity,
            None => return,
        };

        let drag_offset_world = {
            let dragged_widget = ctx.get_widget(dragged_entity.entity);
            let widget_pos = Self::thickness_to_point(*dragged_widget.get::<Thickness>("margin"));

            widget_pos + self.drag_offset
        };

        if self.mouse_position.distance(drag_offset_world) > DRAG_THRESHOLD {
            self.dragging = true;
            ctx.get_widget(dragged_entity.entity)
                .set::<bool>("enabled", false);
        }

        if !self.dragging {
            return;
        }

        match dragged_entity.widget_type {
            WidgetType::Node => {
                self.refresh_node(ctx, dragged_entity.entity);
            }
            WidgetType::Slot => {
                self.grab_slot_edge(ctx, dragged_entity.entity);
            }
            WidgetType::Edge => {
                self.refresh_dragged_edges(ctx);
            }
        };
    }

    fn handle_dropped_entity(&mut self, ctx: &mut Context) {
        if let Some(action) = ctx.widget().get::<OptionAction>("action") {
            match *action {
                Action::Release(_) => {}
                _ => return,
            };
        } else {
            return;
        }
        self.reset_dragging(ctx);


        let dropped_on_entity = match self.dropped_on_entity {
            Some(drag_drop_entity) => drag_drop_entity,
            None => {
                self.remove_dragged_edges(ctx);
                self.update_dragged_node_to_graph(ctx);
                return;
            }
        };

        match dropped_on_entity.widget_type {
            WidgetType::Slot => {
                let dropped_on_widget = ctx.get_widget(dropped_on_entity.entity);

                let dropped_on_node_id = *dropped_on_widget.get::<u32>("node_id");
                let dropped_on_side = *dropped_on_widget.get::<WidgetSide>("side");
                let dropped_on_slot = *dropped_on_widget.get::<u32>("slot_id");

                let goal_position = {
                    let node_margin = *ctx
                        .child(&*dropped_on_node_id.to_string())
                        .get::<Thickness>("my_margin");
                    let node_pos = Point {
                        x: node_margin.left,
                        y: node_margin.top,
                    };
                    Self::position_edge(dropped_on_side, dropped_on_slot, node_pos)
                };

                for edge_entity in self.get_dragged_edges(ctx) {
                    let mut edge_widget = ctx.get_widget(edge_entity);

                    let (other_node_id, other_slot_id, other_side) = match dropped_on_side {
                        WidgetSide::Input => {
                            edge_widget.set::<u32>("input_node", dropped_on_node_id);
                            edge_widget.set::<u32>("input_slot", dropped_on_slot);
                            edge_widget.set::<Point>("input_point", goal_position);
                            (
                                *edge_widget.get::<u32>("output_node"),
                                *edge_widget.get::<u32>("output_slot"),
                                Side::Output,
                            )
                        }
                        WidgetSide::Output => {
                            edge_widget.set::<u32>("output_node", dropped_on_node_id);
                            edge_widget.set::<u32>("output_slot", dropped_on_slot);
                            edge_widget.set::<Point>("output_point", goal_position);
                            (
                                *edge_widget.get::<u32>("input_node"),
                                *edge_widget.get::<u32>("input_slot"),
                                Side::Input,
                            )
                        }
                    };

                    ctx.push_event(ChangedEvent(edge_entity, "".to_string()));
                    let _ = self.node_graph_spatial.node_graph.connect_arbitrary(
                        NodeId(dropped_on_node_id),
                        dropped_on_side.into(),
                        SlotId(dropped_on_slot),
                        NodeId(other_node_id),
                        other_side,
                        SlotId(other_slot_id),
                    );
                }
                self.update_slot_edges_from_graph(ctx, dropped_on_entity.entity);
            }
            WidgetType::Node => {
                panic!("Somehow dropped something on a node, should not be possible")
            }
            WidgetType::Edge => {
                panic!("Somehow dropped something on an edge, should not be possible")
            }
        };

        self.dropped_on_entity = None;
    }

    fn reset_dragging(&mut self, ctx: &mut Context) {
        self.dragging = false;
        if let Some(dragged_entity) = self.dragged_entity {
            ctx.get_widget(dragged_entity.entity)
                .set::<bool>("enabled", true);
        }
        self.drag_offset = DRAG_OFFSET_DEFAULT;
    }

    fn thickness_to_point(thickness: Thickness) -> Point {
        Point::new(thickness.left, thickness.top)
    }

    fn select_entity(&mut self, ctx: &mut Context, option_drag_drop_entity: OptionDragDropEntity) {
        if let Some(drag_drop_entity) = self.selected_entity {
            ctx.get_widget(drag_drop_entity.entity)
                .set::<bool>("selected", false);
        }

        self.selected_entity = if let Some(drag_drop_entity) = option_drag_drop_entity {
            match drag_drop_entity.widget_type {
                WidgetType::Node => {
                    ctx.get_widget(drag_drop_entity.entity)
                        .set::<bool>("selected", true);
                    option_drag_drop_entity
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn add_node(&mut self, ctx: &mut Context, node_type: NodeType) {
        let node_id = self
            .node_graph_spatial
            .node_graph
            .add_node(CoreNode::new(node_type))
            .unwrap();

        self.populate_node(ctx, node_id);

        // self.dragged_entity = Some(DragDropEntity {
        //     entity: Self::get_most_recent_entity_type(ctx, WidgetType::Node),
        //     widget_type: WidgetType::Node,
        // });
    }

    fn reset_mouse_action(&mut self, ctx: &mut Context) {
        if let Some(action) = ctx.widget().get::<OptionAction>("action") {
            if let Action::Release(_) = action {
                self.select_entity(ctx, self.dragged_entity);
                self.dragged_entity = None
            }
        }

        ctx.widget().set::<OptionAction>("action", None);
    }

    fn update_dragged_node_to_graph(&mut self, ctx: &mut Context) {
        let dragged_entity = if let Some(drag_drop_entity) = self.dragged_entity {
            if drag_drop_entity.widget_type == WidgetType::Node {
                drag_drop_entity.entity
            } else {
                return;
            }
        } else {
            return;
        };

        self.update_node_to_graph(ctx, dragged_entity);
    }

    fn update_node_to_graph(&mut self, ctx: &mut Context<'_>, entity: Entity) {
        let widget = ctx.get_widget(entity);

        let margin = widget.get::<Thickness>("margin");

        let node_id = NodeId(*widget.get::<u32>("node_id"));

        for mut location in &mut self.node_graph_spatial.locations {
            if location.node_id == node_id {
                location.point.0 = margin.left;
                location.point.1 = margin.top;
            }
        }
    }

    /// Updates all edges connected to the given `slot_entity` using the data in the graph.
    fn update_slot_edges_from_graph(&mut self, ctx: &mut Context, slot_entity: Entity) {
        Self::delete_edges_in_slot(ctx, slot_entity);
        let slot_widget = ctx.get_widget(slot_entity);

        let node_id = *slot_widget.get::<u32>("node_id");
        let slot_id = *slot_widget.get::<u32>("slot_id");
        let side: Side = (*slot_widget.get::<WidgetSide>("side")).into();

        let edges_to_create: Vec<CoreEdge> = self
            .node_graph_spatial
            .node_graph
            .edges_in_slot(NodeId(node_id), side, SlotId(slot_id))
            .iter()
            .map(|(_, edge)| **edge)
            .collect();

        for edge in edges_to_create {
            self.create_edge(ctx, &edge);
        }
    }

    /// Removes all visual edges connected to a slot.
    fn delete_edges_in_slot(ctx: &mut Context, slot_entity: Entity) {
        if !Self::entity_type(ctx, slot_entity, WidgetType::Slot) {
            return;
        }

        let edge_entities = Self::get_edges_in_slot(ctx, slot_entity);

        for edge in edge_entities {
            ctx.remove_child(edge);
        }
    }

    fn remove_dragged_edges(&mut self, ctx: &mut Context) {
        let dragged_edge_entities: Vec<Entity> = self.get_dragged_edges(ctx);

        for dragged_edge_entity in dragged_edge_entities {
            let dragged_edge_widget = ctx.get_widget(dragged_edge_entity);

            let (output_node, input_node, output_slot, input_slot): (
                NodeId,
                NodeId,
                SlotId,
                SlotId,
            ) = {
                (
                    NodeId(*dragged_edge_widget.get::<u32>("output_node")),
                    NodeId(*dragged_edge_widget.get::<u32>("input_node")),
                    SlotId(*dragged_edge_widget.get::<u32>("output_slot")),
                    SlotId(*dragged_edge_widget.get::<u32>("input_slot")),
                )
            };

            self.node_graph_spatial.node_graph.remove_edge(
                output_node,
                input_node,
                output_slot,
                input_slot,
            );
            ctx.remove_child(dragged_edge_entity);
        }

        self.dragged_edges.0 = Vec::new();
    }

    fn entity_type(ctx: &mut Context, entity: Entity, widget_type_input: WidgetType) -> bool {
        if let Some(widget_type) = ctx.get_widget(entity).try_get::<WidgetType>("widget_type") {
            *widget_type == widget_type_input
        } else {
            false
        }
    }

    /// Updates the visual position of the given `Entity` with `WidgetType::Node`.
    fn refresh_node(&mut self, ctx: &mut Context, node_entity: Entity) {
        if !Self::entity_type(ctx, node_entity, WidgetType::Node) {
            return;
        }
        let mut dragged_widget = ctx.get_widget(node_entity);
        let current_margin = *dragged_widget.get::<Thickness>("my_margin");

        dragged_widget.set::<Thickness>(
            "my_margin",
            Thickness {
                left: self.mouse_position.x() - self.drag_offset.x(),
                right: current_margin.right,
                top: self.mouse_position.y() - self.drag_offset.y(),
                bottom: current_margin.bottom,
            },
        );

        self.refresh_node_edges(ctx, node_entity);
        self.refresh_node_slots(ctx, node_entity);
    }

    fn grab_slot_edge(&mut self, ctx: &mut Context, slot_entity: Entity) {
        let slot_side = *ctx.get_widget(slot_entity).get::<WidgetSide>("side");
        let slot_node_id = *ctx.get_widget(slot_entity).get::<u32>("node_id");
        let slot_id = *ctx.get_widget(slot_entity).get::<u32>("slot_id");

        let mouse_position = Point {
            x: self.mouse_position.x(),
            y: self.mouse_position.y(),
        };

        let dragged_edges = match slot_side {
            WidgetSide::Input => {
                let dragged_edges = self.get_dragged_edges(ctx);

                if dragged_edges.is_empty() {
                    (
                        vec![self.create_loose_edge(
                            ctx,
                            slot_node_id,
                            slot_side,
                            slot_id,
                            None,
                            None,
                            Some(mouse_position),
                        )],
                        WidgetSide::Output,
                    )
                } else {
                    (dragged_edges, WidgetSide::Input)
                }
            }
            WidgetSide::Output => (
                vec![self.create_loose_edge(
                    ctx,
                    slot_node_id,
                    slot_side,
                    slot_id,
                    None,
                    None,
                    Some(mouse_position),
                )],
                WidgetSide::Input,
            ),
        };

        self.dragged_entity = Some(DragDropEntity::new(WidgetType::Edge, Entity(0)));

        self.dragged_edges = dragged_edges;
        self.refresh_dragged_edges(ctx);
    }

    fn refresh_dragged_edges(&mut self, ctx: &mut Context) {
        let mouse_point = Point {
            x: self.mouse_position.x(),
            y: self.mouse_position.y(),
        };
        for edge_entity in self.dragged_edges.0.clone() {
            self.move_edge_side(ctx, edge_entity, self.dragged_edges.1, mouse_point);
        }
    }

    fn move_edge_side(
        &mut self,
        ctx: &mut Context,
        edge_entity: Entity,
        side: WidgetSide,
        position: Point,
    ) {
        let side_string = match side {
            WidgetSide::Input => "input_point",
            WidgetSide::Output => "output_point",
        };

        ctx.get_widget(edge_entity)
            .set::<Point>(side_string, position);
    }

    fn get_dragged_edges(&mut self, ctx: &mut Context) -> Vec<Entity> {
        let dragged_entity = if self.dragged_entity.is_some() {
            self.dragged_entity.unwrap()
        } else {
            return Vec::new();
        };

        match dragged_entity.widget_type {
            WidgetType::Slot => Self::get_edges_in_slot(ctx, dragged_entity.entity),
            WidgetType::Edge => self.dragged_edges.0.clone(),
            _ => Vec::new(),
        }
    }

    fn get_edges_in_slot(ctx: &mut Context, slot_entity: Entity) -> Vec<Entity> {
        let slot_widget = ctx.get_widget(slot_entity);

        let (slot_node_id, slot_id, slot_side) = {
            (
                *slot_widget.get::<u32>("node_id"),
                *slot_widget.get::<u32>("slot_id"),
                *slot_widget.get::<WidgetSide>("side"),
            )
        };

        Self::children_type(ctx, WidgetType::Edge)
            .iter()
            .filter(|entity| {
                let edge_widget = ctx.get_widget(**entity);

                let (
                    edge_output_node_id,
                    edge_input_node_id,
                    edge_output_slot_id,
                    edge_input_slot_id,
                ) = {
                    (
                        *edge_widget.get::<u32>("output_node"),
                        *edge_widget.get::<u32>("input_node"),
                        *edge_widget.get::<u32>("output_slot"),
                        *edge_widget.get::<u32>("input_slot"),
                    )
                };

                match slot_side {
                    WidgetSide::Input => {
                        slot_node_id == edge_input_node_id && slot_id == edge_input_slot_id
                    }
                    WidgetSide::Output => {
                        slot_node_id == edge_output_node_id && slot_id == edge_output_slot_id
                    }
                }
            })
            .copied()
            .collect()
    }

    fn create_loose_edge(
        &mut self,
        ctx: &mut Context,
        node_id: u32,
        side: WidgetSide,
        slot_id: u32,
        other_node_id: Option<u32>,
        other_slot_id: Option<u32>,
        other_point: Option<Point>,
    ) -> Entity {
        let node_margin = *ctx
            .child(&*node_id.to_string())
            .get::<Thickness>("my_margin");
        let node_pos = Point {
            x: node_margin.left,
            y: node_margin.top,
        };
        let slot_position = Self::position_edge(side, slot_id, node_pos);

        let self_entity = ctx.widget().entity();
        let bc = &mut ctx.build_context();
        let item = match side {
            WidgetSide::Input => Edge::new()
                .id("edge")
                .output_point(other_point.unwrap_or_default())
                .input_point(slot_position)
                .output_node(other_node_id.unwrap_or_default())
                .input_node(node_id)
                .output_slot(other_slot_id.unwrap_or_default())
                .input_slot(slot_id)
                .build(bc),
            WidgetSide::Output => Edge::new()
                .id("edge")
                .output_point(slot_position)
                .input_point(other_point.unwrap_or_default())
                .output_node(node_id)
                .input_node(other_node_id.unwrap_or_default())
                .output_slot(slot_id)
                .input_slot(other_slot_id.unwrap_or_default())
                .build(bc),
        };
        bc.append_child(self_entity, item);

        Self::get_most_recent_entity_type(ctx, WidgetType::Edge)
    }

    fn get_most_recent_entity_type(ctx: &mut Context, widget_type: WidgetType) -> Entity {
        *Self::children_type(ctx, widget_type)
            .iter()
            .rev()
            .next()
            .unwrap()
    }

    /// Visually refreshes all `Edge` widgets connected to the given `Entity` based on what's
    /// seen in the GUI, not from the actual data.
    fn refresh_node_edges(&mut self, ctx: &mut Context, node_entity: Entity) {
        let node_widget = ctx.get_widget(node_entity);
        let node_id = *node_widget.get::<u32>("node_id");
        let edge_entities: Vec<Entity> = self.node_edges(ctx, NodeId(node_id));

        for edge_entity in edge_entities {
            let (output_node, input_node, output_slot, input_slot) = {
                let edge_widget = ctx.get_widget(edge_entity);
                (
                    *edge_widget.get::<u32>("output_node"),
                    *edge_widget.get::<u32>("input_node"),
                    *edge_widget.get::<u32>("output_slot"),
                    *edge_widget.get::<u32>("input_slot"),
                )
            };

            let node_point = Self::node_point(ctx, node_entity);

            let mut edge_widget = ctx.get_widget(edge_entity);
            if output_node == node_id {
                edge_widget.set(
                    "output_point",
                    Self::position_edge(WidgetSide::Output, output_slot, node_point),
                );
            } else if input_node == node_id {
                edge_widget.set(
                    "input_point",
                    Self::position_edge(WidgetSide::Input, input_slot, node_point),
                );
            }
        }
    }

    fn node_point(ctx: &mut Context, node_entity: Entity) -> Point {
        let node_widget = ctx.get_widget(node_entity);
        let node_margin = node_widget.get::<Thickness>("my_margin");

        Point {
            x: node_margin.left,
            y: node_margin.top,
        }
    }

    fn refresh_node_slots(&mut self, ctx: &mut Context, node_entity: Entity) {
        if !Self::entity_type(ctx, node_entity, WidgetType::Node) {
            return;
        }
        let node_widget = ctx.get_widget(node_entity);
        let node_margin = *node_widget.get::<Thickness>("margin");

        let node_id = *node_widget.get::<u32>("node_id");
        let slot_entities: Vec<Entity> = self.node_slots(ctx, NodeId(node_id));

        for slot_entity in slot_entities {
            let (slot_id, side) = {
                let slot_widget = ctx.get_widget(slot_entity);
                (
                    *slot_widget.get::<u32>("slot_id"),
                    *slot_widget.get::<WidgetSide>("side"),
                )
            };

            let mut slot_widget = ctx.get_widget(slot_entity);

            slot_widget.set("margin", Self::position_slot(side, slot_id, node_margin));
        }
    }

    fn node_edges(&mut self, ctx: &mut Context, node_id: NodeId) -> Vec<Entity> {
        Self::children_type(ctx, WidgetType::Edge)
            .iter()
            .filter(|entity| {
                let widget = ctx.get_widget(**entity);
                let output_node = *widget.get::<u32>("output_node");
                let input_node = *widget.get::<u32>("input_node");

                output_node == node_id.0 || input_node == node_id.0
            })
            .copied()
            .collect()
    }

    fn node_slots(&mut self, ctx: &mut Context, node_id: NodeId) -> Vec<Entity> {
        Self::children_type(ctx, WidgetType::Slot)
            .iter()
            .filter(|entity| {
                let widget = ctx.get_widget(**entity);
                let slot_node_id = *widget.get::<u32>("node_id");

                slot_node_id == node_id.0
            })
            .copied()
            .collect()
    }

    fn children_type(ctx: &mut Context, widget_type: WidgetType) -> Vec<Entity> {
        let mut output: Vec<Entity> = Vec::new();

        for i in 0.. {
            if let Some(widget) = ctx.try_child_from_index(i) {
                let entity = widget.entity();

                if Self::entity_type(ctx, entity, widget_type) {
                    output.push(entity)
                } else {
                    continue;
                }
            } else {
                break;
            };
        }

        output
    }

    fn position_slot(side: WidgetSide, slot: u32, node_margin: Thickness) -> Thickness {
        let left = node_margin.left - SLOT_SIZE_HALF;
        let top = node_margin.top + ((SLOT_SIZE + SLOT_SPACING) * slot as f64);
        match side {
            WidgetSide::Input => Thickness {
                left,
                top,
                right: 0.,
                bottom: 0.,
            },
            WidgetSide::Output => Thickness {
                left: left + NODE_WIDTH,
                top,
                right: 0.,
                bottom: 0.,
            },
        }
    }

    fn child_entities_type(ctx: &mut Context, widget_type: WidgetType) -> Vec<Entity> {
        child_entities(ctx)
            .iter()
            .filter(|entity| {
                ctx.get_widget(**entity)
                    .try_get::<WidgetType>("widget_type")
                    == Some(&widget_type)
            })
            .cloned()
            .collect()
    }

    fn delete_node(&mut self, ctx: &mut Context, entity: Entity) {
        if !Self::entity_type(ctx, entity, WidgetType::Node) {
            return;
        }

        let node_id = *ctx.get_widget(entity).get::<u32>("node_id");

        // Delete node in graph
        self.node_graph_spatial
            .node_graph
            .remove_node(NodeId(node_id));

        // Delete connected edges in GUI
        Self::disconnect_node(ctx, entity);

        // Delete node in GUI
        ctx.remove_child(entity);
    }

    fn disconnect_node(ctx: &mut Context, entity: Entity) {
        if !Self::entity_type(ctx, entity, WidgetType::Node) {
            return;
        }

        for slot_entity in Self::get_slots_in_node(ctx, entity) {
            Self::delete_slot(ctx, slot_entity);
        }
    }

    fn get_slots_in_node(ctx: &mut Context, entity: Entity) -> Vec<Entity> {
        if !Self::entity_type(ctx, entity, WidgetType::Node) {
            return Vec::new();
        }

        let node_id = *ctx.get_widget(entity).get::<u32>("node_id");

        Self::children_type(ctx, WidgetType::Slot)
            .iter()
            .filter(|entity| *ctx.get_widget(**entity).get::<u32>("node_id") == node_id)
            .copied()
            .collect()
    }

    fn delete_slot(ctx: &mut Context, entity: Entity) {
        if !Self::entity_type(ctx, entity, WidgetType::Slot) {
            return;
        }

        Self::delete_edges_in_slot(ctx, entity);
        ctx.remove_child(entity);
    }

    fn is_clickable(ctx: &mut Context, entity: Entity) -> bool {
        if let Some(widget_type) = ctx.get_widget(entity).try_get("widget_type") {
            match widget_type {
                WidgetType::Node => true,
                WidgetType::Slot => true,
                _ => false,
            }
        } else {
            false
        }
    }

    fn handle_action_main(&mut self, ctx: &mut Context<'_>) {
        if let Some(action_main) = ctx.widget().get::<OptionActionMain>("action_main").clone() {
            match action_main {
                ActionMain::LoadGraph(path) => {
                    self.load_graph(ctx, path);
                }
                ActionMain::SaveGraph(path) => {
                    self.save_graph(path);
                }
                _ => {}
            };

            ctx.widget().set::<OptionActionMain>("action_main", None);
        }
    }

    fn populate_workspace(&mut self, ctx: &mut Context<'_>) {
        ctx.clear_children();
        self.init_menu_property(ctx);

        self.populate_nodes(ctx);
        self.populate_slots(ctx);
        self.populate_edges(ctx);
    }

    fn try_get_location(&self, _ctx: &mut Context, node_id: NodeId) -> Option<(f64, f64)> {
        match self
            .node_graph_spatial
            .locations
            .iter()
            .find(|loc| loc.node_id == node_id)
        {
            Some(location) => Some(location.point),
            None => None,
        }
    }

    fn populate_node(&mut self, ctx: &mut Context, node_id: NodeId) {
        let node = self
            .node_graph_spatial
            .node_graph
            .node_with_id(node_id)
            .unwrap();
        let node_type = &node.node_type;
        let input_capacity = node.capacity(Side::Input);
        let outputput_capacity = node.capacity(Side::Output);

        let location_point = match self.try_get_location(ctx, node_id) {
            Some(location_point) => location_point,
            None => (0., 0.),
        };

        let location = Location {
            node_id,
            point: location_point,
        };
        self.node_graph_spatial.locations.push(location);

        let node_title = format!("{:?}", node_type);

        let margin = Thickness {
            left: location_point.0,
            top: location_point.1,
            right: 0.,
            bottom: 0.,
        };

        let slot_count_input = match node_type {
            NodeType::InputGray | NodeType::InputRgba => 0,
            _ => input_capacity,
        };
        let slot_count_output = match node_type {
            NodeType::OutputGray | NodeType::OutputRgba => 0,
            _ => outputput_capacity,
        };

        let self_entity = ctx.widget().entity();
        let bc = &mut ctx.build_context();

        let item = Node::new()
            .id(node_id.0.to_string())
            .title(node_title)
            .node_id(node_id.0)
            .my_margin(margin)
            .slot_count_input(slot_count_input)
            .slot_count_output(slot_count_output)
            .build(bc);

        bc.append_child(self_entity, item);

        let created_node_entity = Self::get_most_recent_entity_type(ctx, WidgetType::Node);
        self.populate_node_slots(ctx, created_node_entity);
    }

    fn populate_nodes(&mut self, ctx: &mut Context) {
        for node_id in self.node_graph_spatial.node_graph.node_ids() {
            self.populate_node(ctx, node_id);
        }
    }

    fn populate_node_slots(&mut self, ctx: &mut Context, node_entity: Entity) {
        let self_entity = ctx.widget().entity();
        let node_margin = *ctx.get_widget(node_entity).get::<Thickness>("my_margin");
        let node_id = *ctx.get_widget(node_entity).get::<u32>("node_id");

        for i in 0..*ctx.get_widget(node_entity).get::<usize>("slot_count_input") {
            let build_context = &mut ctx.build_context();

            let slot_margin = Self::position_slot(WidgetSide::Input, i as u32, node_margin);

            let item = Slot::new()
                .node_id(node_id)
                .margin(slot_margin)
                .side(WidgetSide::Input)
                .slot_id(i as u32)
                .build(build_context);

            build_context.append_child(self_entity, item);
        }

        for i in 0..*ctx
            .get_widget(node_entity)
            .get::<usize>("slot_count_output")
        {
            let build_context = &mut ctx.build_context();

            let slot_margin = Self::position_slot(WidgetSide::Output, i as u32, node_margin);

            let item = Slot::new()
                .node_id(node_id)
                .margin(slot_margin)
                .side(WidgetSide::Output)
                .slot_id(i as u32)
                .build(build_context);

            build_context.append_child(self_entity, item);
        }
    }

    fn populate_slots(&mut self, ctx: &mut Context) {
        for node_entity in Self::child_entities_type(ctx, WidgetType::Node) {
            self.populate_node_slots(ctx, node_entity);
        }
    }

    fn populate_edges(&mut self, ctx: &mut Context) {
        for edge in self.node_graph_spatial.node_graph.edges.clone() {
            self.create_edge(ctx, &edge);
        }
    }

    fn create_edge(&mut self, ctx: &mut Context, edge: &CoreEdge) {
        let self_entity = ctx.widget().entity();
        let bc = &mut ctx.build_context();

        let output_node_pos = self
            .node_graph_spatial
            .locations
            .iter()
            .find(|loc| loc.node_id == edge.output_id)
            .expect("Could not find output node location")
            .point;
        let output_node_pos = Point {
            x: output_node_pos.0,
            y: output_node_pos.1,
        };

        let input_node_pos = self
            .node_graph_spatial
            .locations
            .iter()
            .find(|loc| loc.node_id == edge.input_id)
            .expect("Could not find input node location")
            .point;
        let input_node_pos = Point {
            x: input_node_pos.0,
            y: input_node_pos.1,
        };

        let output_slot = edge.output_slot.0;
        let input_slot = edge.input_slot.0;

        let output_point = Self::position_edge(WidgetSide::Output, output_slot, output_node_pos);
        let input_point = Self::position_edge(WidgetSide::Input, input_slot, input_node_pos);

        let item = Edge::new()
            .id("edge")
            .output_point(output_point)
            .input_point(input_point)
            .output_node(edge.output_id.0)
            .input_node(edge.input_id.0)
            .output_slot(output_slot)
            .input_slot(input_slot)
            .build(bc);

        bc.append_child(self_entity, item);
    }

    fn position_edge(side: WidgetSide, slot: u32, node_position: Point) -> Point {
        let x = node_position.x();
        let y = node_position.y() + SLOT_SIZE_HALF + ((SLOT_SIZE + SLOT_SPACING) * slot as f64);
        match side {
            WidgetSide::Input => Point { x, y },
            WidgetSide::Output => Point {
                x: x + NODE_WIDTH,
                y,
            },
        }
    }

    fn load_graph(&mut self, ctx: &mut Context<'_>, path: String) {
        let file = File::open(path).unwrap();
        self.node_graph_spatial = serde_json::from_reader(file).unwrap();

        self.dragged_edges.0 = Vec::new();
        self.populate_workspace(ctx);
    }

    fn save_graph(&mut self, path: String) {
        let file = File::create(path).unwrap();
        serde_json::to_writer_pretty(&file, &self.node_graph_spatial).unwrap();
    }
}
