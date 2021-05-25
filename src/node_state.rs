use bevy::prelude::*;
use kanter_core::{engine::NodeState, node_graph::NodeId};

use crate::{sync_graph::SLOT_SIZE, thumbnail::THUMBNAIL_SIZE, Stage};

struct StateMaterials {
    clean: Handle<ColorMaterial>,
    dirty: Handle<ColorMaterial>,
    requested: Handle<ColorMaterial>,
    prioritised: Handle<ColorMaterial>,
    processing: Handle<ColorMaterial>,
}

impl StateMaterials {
    fn from_node_state(&self, node_state: NodeState) -> Handle<ColorMaterial> {
        match node_state {
            NodeState::Clean => &self.clean,
            NodeState::Dirty => &self.dirty,
            NodeState::Requested => &self.requested,
            NodeState::Prioritised => &self.prioritised,
            NodeState::Processing => &self.processing,
        }
        .clone()
    }
}

struct StateImage;

pub(crate) struct NodeStatePlugin;

impl Plugin for NodeStatePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .after(Stage::Apply)
                    .with_system(add_state_image.system().chain(state_materials.system())),
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(StateMaterials {
        clean: materials.add(asset_server.load("image/node_states/clean.png").into()),
        dirty: materials.add(asset_server.load("image/node_states/dirty.png").into()),
        requested: materials.add(asset_server.load("image/node_states/requested.png").into()),
        prioritised: materials.add(
            asset_server
                .load("image/node_states/prioritised.png")
                .into(),
        ),
        processing: materials.add(asset_server.load("image/node_states/processing.png").into()),
    });
}

fn add_state_image(
    q_node: Query<(Entity, &NodeState), Added<NodeState>>,
    mut commands: Commands,
    materials: Res<StateMaterials>,
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
                    material: materials.from_node_state(*node_state),
                    sprite: Sprite::new(Vec2::new(SLOT_SIZE, SLOT_SIZE)),
                    ..Default::default()
                })
                .insert(StateImage);
        });
    }
}

fn state_materials(
    q_node: Query<(Entity, &NodeState), Changed<NodeState>>,
    mut q_state_image: Query<(&Parent, &mut Handle<ColorMaterial>), With<StateImage>>,
    materials: Res<StateMaterials>,
) {
    for (node_e, node_state) in q_node.iter() {
        if let Some((_, mut color_material)) = q_state_image
            .iter_mut()
            .find(|(parent, _)| parent.0 == node_e)
        {
            *color_material = materials.from_node_state(*node_state);
        }
    }
}
