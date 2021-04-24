#![allow(clippy::type_complexity)] // Avoids many warnings about very complex types.
pub mod add_tool;
pub mod box_select;
pub mod camera;
pub mod mouse_interaction;
pub mod processing;
pub mod scan_code_input;
pub mod drag_drop_entity;
pub mod workspace;
pub mod material;
pub mod sync_graph;
pub mod instructions;
pub mod deselect_tool;
pub mod delete_tool;
pub mod hotkeys;

use bevy::{audio::AudioPlugin, prelude::*};
use add_tool::*;
use box_select::*;
use camera::*;
use mouse_interaction::*;
use processing::*;
use scan_code_input::*;
use drag_drop_entity::*;
use workspace::*;
use material::*;
use sync_graph::*;
use instructions::*;
use deselect_tool::*;
use delete_tool::*;
use hotkeys::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum GrabToolType {
    Add,
    Node,
    Slot,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ToolState {
    None,
    Add,
    BoxSelect,
    Delete,
    Export,
    Grab(GrabToolType),
    Process,
}

impl Default for ToolState {
    fn default() -> Self {
        Self::None
    }
}

fn main() {
    App::build()
        .insert_resource(WindowDescriptor {
            title: "Kanter".to_string(),
            width: 1024.0,
            height: 768.0,
            vsync: false,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.5)))
        // .insert_resource(bevy::ecs::schedule::ReportExecutionOrderAmbiguities)
        .add_plugins_with(DefaultPlugins, |group| group.disable::<AudioPlugin>())
        .add_plugin(KanterPlugin)
        .run();
}

struct Crosshair;
struct Cursor;

struct Hoverable;
struct Hovered;

struct Selected;

struct Draggable;
struct Dragged;
struct Dropped;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum Stage {
    Input,
    Update,
    Apply,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, AmbiguitySetLabel)]
struct AmbiguitySet;

pub struct KanterPlugin;

impl Plugin for KanterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_state(ToolState::None)
            .add_state(FirstPersonState::Off)
            .add_plugin(ScanCodeInputPlugin)
            .add_plugin(AddToolPlugin)
            .add_plugin(WorkspaceDragDropPlugin)
            .add_plugin(ProcessingPlugin)
            .add_plugin(MouseInteractionPlugin)
            .add_plugin(BoxSelectPlugin)
            .add_plugin(CameraPlugin)
            .add_plugin(WorkspacePlugin)
            .add_plugin(MaterialPlugin)
            .add_plugin(SyncGraphPlugin)
            .add_plugin(InstructionPlugin)
            .add_plugin(DeselectToolPlugin)
            .add_plugin(DeleteToolPlugin)
            .add_plugin(HotkeysPlugin)
            .add_startup_system(setup.system())
            .add_system_set_to_stage(
                CoreStage::Update,
                SystemSet::new()
                    .label(Stage::Update)
                    .after(Stage::Input)
                    .with_system(
                        hoverable
                            .system()
                            .with_run_criteria(State::on_update(ToolState::None))
                            .in_ambiguity_set(AmbiguitySet),
                    ),
            );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let crosshair_image = asset_server.load("crosshair.png");

    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(WorkspaceCamera)
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Transform::from_translation(Vec3::new(
                    0.,
                    0.,
                    -CAMERA_DISTANCE,
                )))
                .insert(GlobalTransform::default())
                .insert(WorkspaceCameraAnchor)
                .with_children(|parent| {
                    parent
                        .spawn_bundle(SpriteBundle {
                            material: materials.add(crosshair_image.into()),
                            visible: Visible {
                                is_visible: false,
                                is_transparent: true,
                            },
                            ..Default::default()
                        })
                        .insert(Transform::from_translation(Vec3::new(0., 0., 9.0)))
                        .insert(Crosshair);
                });
        });
    commands
        .spawn()
        .insert(Transform::default())
        .insert(GlobalTransform::default())
        .insert(Cursor);
}

fn hoverable(
    mut commands: Commands,
    workspace: Res<Workspace>,
    q_hoverable: Query<(Entity, &GlobalTransform, &Sprite), (With<Hoverable>, Without<Dragged>)>,
) {
    if workspace.cursor_moved {
        for (entity, global_transform, sprite) in q_hoverable.iter() {
            if box_contains_point(
                global_transform.translation.truncate(),
                sprite.size,
                workspace.cursor_world,
            ) {
                commands.entity(entity).insert(Hovered);
            } else {
                commands.entity(entity).remove::<Hovered>();
            }
        }
    }
}

fn box_contains_point(box_pos: Vec2, box_size: Vec2, point: Vec2) -> bool {
    let half_size = box_size / 2.;

    box_pos.x - half_size.x < point.x
        && box_pos.x + half_size.x > point.x
        && box_pos.y - half_size.y < point.y
        && box_pos.y + half_size.y > point.y
}

fn stretch_between(sprite: &mut Sprite, transform: &mut Transform, start: Vec2, end: Vec2) {
    let midpoint = start - (start - end) / 2.;
    let distance = start.distance(end);
    let rotation = Vec2::X.angle_between(start - end);

    transform.translation = midpoint.extend(0.0);
    transform.rotation = Quat::from_rotation_z(rotation);
    sprite.size = Vec2::new(distance, 5.);
}