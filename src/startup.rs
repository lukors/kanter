use std::sync::Arc;

use bevy::prelude::*;
use kanter_core::texture_processor::TextureProcessor;

pub(crate) struct Startup;

impl Plugin for Startup {
    fn build(&self, app: &mut AppBuilder) {
        let tex_pro = TextureProcessor::new(Arc::new(1_000_000_000.into()));

        app.insert_non_send_resource(tex_pro);
    }
}
