use orbtk::prelude::*;

widget!(
    Line<LineState> {
        start_point: Point,
        end_point: Point
    }
);

impl Template for Line {
    fn template(self, _id: Entity, _ctx: &mut BuildContext) -> Self {
        self.name("Line")
            .start_point(Point { x: 0., y: 0. })
            .end_point(Point { x: 0., y: 0. })
    }

    fn render_object(&self) -> Box<dyn RenderObject> {
        Box::new(LineRenderObject)
    }
}

#[derive(AsAny, Default)]
pub struct LineState {}

impl State for LineState {}

pub struct LineRenderObject;

impl RenderObject for LineRenderObject {
    fn render_self(&self, ctx: &mut Context<'_>, global_position: &Point) {
        let (start_point, end_point) = {
            let widget = ctx.widget();
            (
                *widget.get::<Point>("start_point"),
                *widget.get::<Point>("end_point"),
            )
        };

        let rc2d = ctx.render_context_2_d();
        rc2d.begin_path();
        rc2d.set_line_width(3.);
        rc2d.set_stroke_style(Brush::SolidColor(Color::rgb(0, 0, 0)));
        rc2d.move_to(
            global_position.x() + start_point.x(),
            global_position.y() + start_point.y(),
        );
        rc2d.line_to(
            global_position.x() + end_point.x(),
            global_position.y() + end_point.y(),
        );
        rc2d.stroke();
    }
}
