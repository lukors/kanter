use crate::{line::Line, shared::*};
use orbtk::prelude::*;

widget!(
    Edge<EdgeState> {
        widget_type: WidgetType,
        output_point: Point,
        input_point: Point,
        output_node: u32,
        input_node: u32,
        output_slot: u32,
        input_slot: u32
    }
);

impl Template for Edge {
    fn template(self, id: Entity, ctx: &mut BuildContext) -> Self {
        self.name("Edge")
            .id("edge")
            .widget_type(WidgetType::Edge)
            .child(
                Line::new()
                    .start_point(("output_point", id))
                    .end_point(("input_point", id))
                    .build(ctx),
            )
    }
}

#[derive(AsAny, Default)]
pub struct EdgeState {}

impl State for EdgeState {}
