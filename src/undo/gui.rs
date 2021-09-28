use super::undo_command_manager::{UndoCommand, UndoCommandManager};
use crate::core_translation::Translator;
use kanter_core::live_graph::LiveGraph;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct GuiUndoCommand<T, U>
where
    T: Debug + Clone,
    U: Translator<T>,
{
    contact_info: U,
    from: T,
    to: T,
}

impl<T: Debug + Clone, U: Translator<T>> UndoCommand for GuiUndoCommand<T, U> {
    fn forward(&self, world: &mut bevy::prelude::World, _: &mut UndoCommandManager) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                let _ = self.contact_info.set(&mut live_graph, self.to.clone());
            }
        }
    }

    fn backward(&self, world: &mut bevy::prelude::World, _: &mut UndoCommandManager) {
        if let Some(live_graph) = world.get_resource::<Arc<RwLock<LiveGraph>>>() {
            if let Ok(mut live_graph) = live_graph.write() {
                let _ = self.contact_info.set(&mut live_graph, self.from.clone());
            }
        }
    }
}

impl<T, U> GuiUndoCommand<T, U>
where
    T: Debug + Clone,
    U: Translator<T>,
{
    pub fn new(contact_info: U, from: T, to: T) -> Self {
        Self {
            contact_info,
            from,
            to,
        }
    }
}
