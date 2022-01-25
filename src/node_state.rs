use bevy::prelude::*;
use kanter_core::live_graph::NodeState;

use crate::{shared::NodeStateComponent, sync_graph::SLOT_SIZE, thumbnail::THUMBNAIL_SIZE, CustomStage};

struct StateImages {
    clean: Handle<Image>,
    dirty: Handle<Image>,
    requested: Handle<Image>,
    prioritised: Handle<Image>,
    processing: Handle<Image>,
    processing_dirty: Handle<Image>,
}

impl StateImages {
    fn from_node_state(&self, node_state: NodeState) -> Handle<Image> {
        match node_state {
            NodeState::Clean => &self.clean,
            NodeState::Dirty => &self.dirty,
            NodeState::Requested => &self.requested,
            NodeState::Prioritised => &self.prioritised,
            NodeState::Processing => &self.processing,
            NodeState::ProcessingDirty => &self.processing_dirty,
        }
        .clone()
    }
}

#[derive(Component)]
struct StateImage;

pub(crate) struct NodeStatePlugin;

impl Plugin for NodeStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .after(CustomStage::Apply)
                    .with_system(add_state_image.system().chain(state_materials.system())),
            );
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(StateImages {
        clean: asset_server.load("image/node_states/clean.png"),
        dirty: asset_server.load("image/node_states/dirty.png"),
        requested: asset_server.load("image/node_states/requested.png"),
        prioritised: asset_server.load("image/node_states/prioritised.png"),
        processing: asset_server.load("image/node_states/processing.png"),
        processing_dirty: asset_server.load("image/node_states/processing_dirty.png"),
    });
}

fn add_state_image(
    q_node: Query<(Entity, &NodeStateComponent), Added<NodeStateComponent>>,
    mut commands: Commands,
    state_images: Res<StateImages>,
) {
    for (node_e, node_state) in q_node.iter() {
        commands.entity(node_e).with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(
                        THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2.,
                        -THUMBNAIL_SIZE / 2. - SLOT_SIZE / 2.,
                        0.1,
                    )),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                        ..Default::default()
                    },
                    texture: state_images.from_node_state(node_state.0),
                    ..Default::default()
                })
                .insert(StateImage);
        });
    }
}

fn state_materials(
    q_node: Query<(Entity, &NodeStateComponent), Changed<NodeStateComponent>>,
    mut q_state_image: Query<(&Parent, &mut Handle<Image>), With<StateImage>>,
    images: Res<StateImages>,
) {
    for (node_e, node_state) in q_node.iter() {
        if let Some((_, mut color_material)) = q_state_image
            .iter_mut()
            .find(|(parent, _)| parent.0 == node_e)
        {
            *color_material = images.from_node_state(node_state.0);
        }
    }
}
