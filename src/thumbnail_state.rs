use bevy::prelude::*;

use crate::{
    thumbnail::{ThumbnailState, THUMBNAIL_SIZE},
    Stage,
};

struct StateMaterials {
    waiting: Handle<ColorMaterial>,
    missing: Handle<ColorMaterial>,
    processing: Handle<ColorMaterial>,
    present: Handle<ColorMaterial>,
}

impl StateMaterials {
    fn from_thumbnail_state(&self, node_state: ThumbnailState) -> Handle<ColorMaterial> {
        match node_state {
            ThumbnailState::Waiting => &self.waiting,
            ThumbnailState::Missing => &self.missing,
            ThumbnailState::Processing => &self.processing,
            ThumbnailState::Present => &self.present,
        }
        .clone()
    }
}

struct StateImage;

pub(crate) struct ThumbnailStatePlugin;

impl Plugin for ThumbnailStatePlugin {
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
        waiting: materials.add(
            asset_server
                .load("image/thumbnail_states/waiting.png")
                .into(),
        ),
        missing: materials.add(
            asset_server
                .load("image/thumbnail_states/missing.png")
                .into(),
        ),
        processing: materials.add(
            asset_server
                .load("image/thumbnail_states/processing.png")
                .into(),
        ),
        present: materials.add(
            asset_server
                .load("image/thumbnail_states/present.png")
                .into(),
        ),
    });
}

fn add_state_image(
    q_thumbnail: Query<(Entity, &ThumbnailState), Added<ThumbnailState>>,
    mut commands: Commands,
    materials: Res<StateMaterials>,
) {
    for (node_e, thumb_state) in q_thumbnail.iter() {
        info!("Adding state image");
        commands.entity(node_e).with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    material: materials.from_thumbnail_state(*thumb_state),
                    sprite: Sprite::new(Vec2::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)),
                    ..Default::default()
                })
                .insert(StateImage);
        });
    }
}

fn state_materials(
    q_node: Query<(Entity, &ThumbnailState), Changed<ThumbnailState>>,
    mut q_state_image: Query<(&Parent, &mut Handle<ColorMaterial>), With<StateImage>>,
    materials: Res<StateMaterials>,
) {
    for (node_e, node_state) in q_node.iter() {
        if let Some((_, mut color_material)) = q_state_image
            .iter_mut()
            .find(|(parent, _)| parent.0 == node_e)
        {
            *color_material = materials.from_thumbnail_state(*node_state);
        }
    }
}
