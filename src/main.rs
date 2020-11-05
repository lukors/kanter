use crate::{shared::*, workspace::Workspace};
use orbtk::prelude::*;

mod shared;
mod workspace;
mod node_container;
mod menu_property;
mod node;
mod slot;
mod edge;
mod line;

fn main() {
    Application::new()
        .window(|ctx| {
            Window::new()
                .title("Kanter")
                .position((300., 300.))
                .size(1024., 768.)
                .resizeable(true)
                .child(MainView::new().build(ctx))
                .build(ctx)
        })
        .run();
}

widget!(MainView<MainState> {
    node_container_entity: u32
});

#[derive(Default, AsAny)]
pub struct MainState {
    pub option_action_main: OptionActionMain,
    workspace: Entity,
}

impl MainState {
    pub fn action_main(&mut self, option_action_main: impl Into<OptionActionMain>) {
        self.option_action_main = option_action_main.into();
    }

    fn node_container_action(&mut self, ctx: &mut Context) {
        if let Some(action_main) = &self.option_action_main {
            let path = ctx.child("graph_path").clone::<String>("text");

            let action_to_send = match action_main {
                ActionMain::LoadGraph(_) => Some(ActionMain::LoadGraph(path.to_string())),
                ActionMain::SaveGraph(_) => Some(ActionMain::SaveGraph(path.to_string())),
                _ => None,
            };

            let node_container_entity = Entity(*ctx.widget().get::<u32>("node_container_entity"));

            ctx.get_widget(node_container_entity)
                .set::<OptionActionMain>("action_main", action_to_send);
        }
    }

    fn workspace_action(&mut self, ctx: &mut Context) {
        if let Some(action_main) = &self.option_action_main {
            if let ActionMain::MenuNode(p) = action_main {
                ctx.get_widget(self.workspace)
                    .set::<OptionActionMain>("action_main", Some(ActionMain::MenuNode(*p)));
            }
        }
    }
}

impl State for MainState {
    fn update(&mut self, _: &mut Registry, ctx: &mut Context) {
        self.node_container_action(ctx);
        self.workspace_action(ctx);

        self.option_action_main = None;
    }
}

impl Template for MainView {
    fn template(mut self, id: Entity, ctx: &mut BuildContext) -> Self {
        let workspace = Workspace::new().build(ctx);
        self.state_mut().workspace = workspace;

        self.name("MainView").child(workspace).child(
            Stack::new()
                .orientation(Orientation::Horizontal)
                .child(
                    Button::new()
                        .style("button")
                        .on_click(move |states, _| {
                            states
                                .get_mut::<MainState>(id)
                                .action_main(ActionMain::LoadGraph("".to_string()));
                            true
                        })
                        .text("Load graph")
                        .width(100.)
                        .build(ctx),
                )
                .child(
                    Button::new()
                        .style("button")
                        .on_click(move |states, _| {
                            states
                                .get_mut::<MainState>(id)
                                .action_main(ActionMain::SaveGraph("".to_string()));
                            true
                        })
                        .text("Save graph")
                        .width(100.)
                        .build(ctx),
                )
                .child(
                    TextBox::new()
                        .id("graph_path")
                        .text("data/invert_graph.json")
                        .build(ctx),
                )
                .child(
                    Button::new()
                        .style("button")
                        .on_click(move |states, p| {
                            states.send_message(Message::OpenAddNodeMenu, workspace);
                            // states
                            //     .get_mut::<MainState>(id)
                            //     .action_main(ActionMain::MenuNode(p));
                            true
                        })
                        .text("Add node")
                        .width(100.)
                        .build(ctx),
                )
                .build(ctx),
        )
    }
}
