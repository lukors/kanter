use crate::camera::WorkspaceCamera;
use bevy::{input::mouse::MouseMotion, prelude::*};

/// Keeps track of and gives access to all that's going on in the workspace.

const DRAG_THRESHOLD: f32 = 5.;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Drag {
    False,
    Starting,
    True,
    Dropping,
}

impl Default for Drag {
    fn default() -> Self {
        Drag::False
    }
}

#[derive(Default)]
pub(crate) struct Workspace {
    pub cursor_screen: Vec2,
    pub cursor_world: Vec2,
    pub cursor_delta: Vec2,
    pub cursor_moved: bool,
    pub drag: Drag,
}
pub(crate) struct WorkspacePlugin;

impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(Workspace::default())
        .add_system_set_to_stage(
            CoreStage::PreUpdate,
            SystemSet::new().with_system(workspace.system()),
        );
    }
}

fn workspace(
    mut er_mouse_motion: EventReader<MouseMotion>,
    mut er_cursor_moved: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut workspace: ResMut<Workspace>,
    i_mouse_button: Res<Input<MouseButton>>,
    q_camera: Query<&Transform, With<WorkspaceCamera>>,
    mut true_cursor_world: Local<Vec2>,
) {
    let mut event_cursor_delta: Vec2 = Vec2::ZERO;
    for event_motion in er_mouse_motion.iter() {
        event_cursor_delta += event_motion.delta;
    }
    let event_cursor_screen = er_cursor_moved.iter().last();

    if let Some(event_cursor_screen) = event_cursor_screen {
        workspace.cursor_screen = event_cursor_screen.position;

        if let (Some(window), Some(cam_transform)) = (windows.get_primary(), q_camera.iter().last()) {
            *true_cursor_world = cursor_to_world(window, cam_transform, event_cursor_screen.position);
        }

        workspace.cursor_moved = true;
    } else {
        workspace.cursor_moved = false;
    }

    workspace.cursor_delta = event_cursor_delta;

    if !i_mouse_button.pressed(MouseButton::Left) || workspace.drag == Drag::True {
        workspace.cursor_world = *true_cursor_world;
    }

    if workspace.drag == Drag::Dropping {
        workspace.drag = Drag::False;
    } else if workspace.drag == Drag::Starting {
        workspace.drag = Drag::True;
    }

    if i_mouse_button.just_released(MouseButton::Left) && workspace.drag == Drag::True {
        workspace.drag = Drag::Dropping;
    }

    if i_mouse_button.pressed(MouseButton::Left)
        && true_cursor_world.distance(workspace.cursor_world) > DRAG_THRESHOLD
        && workspace.drag == Drag::False
    {
        workspace.drag = Drag::Starting;
    }
}

fn cursor_to_world(window: &Window, cam_transform: &Transform, cursor_pos: Vec2) -> Vec2 {
    // get the size of the window
    let size = Vec2::new(window.width() as f32, window.height() as f32);

    // the default orthographic projection is in pixels from the center;
    // just undo the translation
    let screen_pos = cursor_pos - size / 2.0;

    // apply the camera transform
    let out = cam_transform.compute_matrix() * screen_pos.extend(0.0).extend(1.0);
    Vec2::new(out.x, out.y)
}