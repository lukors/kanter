use crate::shared::*;
use orbtk::prelude::*;

widget!(
    Slot<SlotState> {
        widget_type: WidgetType,
        side: WidgetSide,
        node_workspace: Entity,
        node_id: u32,
        slot_id: u32
    }
);

impl Template for Slot {
    fn template(self, _id: Entity, ctx: &mut BuildContext) -> Self {
        self.name("Slot")
            .widget_type(WidgetType::Slot)
            .width(SLOT_SIZE)
            .height(SLOT_SIZE)
            .child(
                Container::new()
                    .background(Color::rgb(200, 200, 200))
                    .border_width(1.)
                    .border_radius(SLOT_SIZE_HALF)
                    .border_brush(Brush::SolidColor(Color::rgb(0, 0, 0)))
                    .build(ctx),
            )
    }
}

#[derive(Default, AsAny)]
pub struct SlotState {}
impl State for SlotState {}
