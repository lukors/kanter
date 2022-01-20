use bevy::prelude::*;

use crate::{
    thumbnail::{ThumbnailState, THUMBNAIL_SIZE},
    Stage,
};

struct StateImages {
    waiting: Handle<Image>,
    missing: Handle<Image>,
    processing: Handle<Image>,
    present: Handle<Image>,
}

impl StateImages {
    fn from_thumbnail_state(&self, node_state: ThumbnailState) -> Handle<Image> {
        match node_state {
            ThumbnailState::Waiting => &self.waiting,
            ThumbnailState::Missing => &self.missing,
            ThumbnailState::Processing => &self.processing,
            ThumbnailState::Present => &self.present,
        }
        .clone()
    }
}

#[derive(Component)]
struct StateImage;

pub(crate) struct ThumbnailStatePlugin;

impl Plugin for ThumbnailStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup.system());
            // .add_system_set_to_stage(
            //     CoreStage::Update,
            //     SystemSet::new()
            //         .after(Stage::Apply)
            //         .with_system(add_state_image.system().chain(state_materials.system())),
            // );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(StateImages {
        waiting: asset_server.load("image/thumbnail_states/waiting.png"),
        missing: asset_server.load("image/thumbnail_states/missing.png"),
        processing: asset_server.load("image/thumbnail_states/processing.png"),
        present: asset_server.load("image/thumbnail_states/present.png"),
    });
}

fn add_state_image(
    q_thumbnail: Query<(Entity, &ThumbnailState), Added<ThumbnailState>>,
    mut commands: Commands,
    images: Res<StateImages>,
) {
    for (node_e, thumb_state) in q_thumbnail.iter() {
        info!("Adding state image");
        commands.entity(node_e).with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)),
                        ..Default::default()
                    },
                    texture: images.from_thumbnail_state(*thumb_state),
                    // material: materials.from_thumbnail_state(*thumb_state),
                    // sprite: Sprite::new(Vec2::new(THUMBNAIL_SIZE, THUMBNAIL_SIZE)),
                    ..Default::default()
                })
                .insert(StateImage);
        });
    }
}

fn state_images(
    q_node: Query<(Entity, &ThumbnailState), Changed<ThumbnailState>>,
    mut q_state_image: Query<(&Parent, &mut Handle<Image>), With<StateImage>>,
    materials: Res<StateImages>,
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
