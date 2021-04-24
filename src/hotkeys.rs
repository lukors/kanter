use bevy::{app::AppExit, prelude::*, window::WindowFocused};

use crate::{Stage, GrabToolType, ToolState, camera::FirstPersonState, scan_code_input::{ScanCode, ScanCodeInput}};

pub(crate) struct HotkeysPlugin;

impl Plugin for HotkeysPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(Stage::Input)
                .with_system(hotkeys.system())
                // .with_system(print_pressed_keys.system())
                .with_system(focus_change.system()),
        )
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new().with_system(quit_hotkey.system()),
        );
    }
}

fn focus_change(
    mut er_window_focused: EventReader<WindowFocused>,
    mut keyboard_input: ResMut<ScanCodeInput>,
) {
    if er_window_focused.iter().any(|event| !event.focused) {
        keyboard_input.clear();
    }
}

fn quit_hotkey(input: Res<ScanCodeInput>, mut app_exit_events: EventWriter<AppExit>) {
    if control_pressed(&input) && input.just_pressed(ScanCode::KeyQ) {
        app_exit_events.send(AppExit);
    }
}

pub(crate) fn control_pressed(scan_code_input: &Res<ScanCodeInput>) -> bool {
    scan_code_input.pressed(ScanCode::ControlLeft)
        || scan_code_input.pressed(ScanCode::ControlRight)
}
fn shift_pressed(scan_code_input: &Res<ScanCodeInput>) -> bool {
    scan_code_input.pressed(ScanCode::ShiftLeft) || scan_code_input.pressed(ScanCode::ShiftRight)
}
fn alt_pressed(scan_code_input: &Res<ScanCodeInput>) -> bool {
    scan_code_input.pressed(ScanCode::AltLeft) || scan_code_input.pressed(ScanCode::AltRight)
}

#[allow(dead_code)]
fn print_pressed_keys(scan_code_input: Res<ScanCodeInput>) {
    for code in scan_code_input.get_just_pressed() {
        info!("ScanCode: {:?}", code);
    }
}

fn hotkeys(
    mut first_person_state: ResMut<State<FirstPersonState>>,
    mut tool_state: ResMut<State<ToolState>>,
    i_mouse_button: Res<Input<MouseButton>>,
    sc_input: Res<ScanCodeInput>,
) {
    if sc_input.just_pressed(ScanCode::Backquote) {
        if *first_person_state.current() == FirstPersonState::Off {
            first_person_state.set(FirstPersonState::On).unwrap();
        } else {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }

    let tool_current = tool_state.current().clone();

    if tool_current == ToolState::None {
        for key_code in sc_input.get_just_pressed() {
            let new_tool = match key_code {
                ScanCode::Delete | ScanCode::KeyX => Some(tool_state.set(ToolState::Delete)),
                ScanCode::F12 => Some(tool_state.set(ToolState::Process)),
                ScanCode::KeyA => {
                    if shift_pressed(&sc_input) {
                        Some(tool_state.set(ToolState::Add))
                    } else {
                        None
                    }
                }
                ScanCode::KeyG => Some(tool_state.set(ToolState::Grab(GrabToolType::Node))),
                ScanCode::KeyS => {
                    if alt_pressed(&sc_input) && shift_pressed(&sc_input) {
                        Some(tool_state.set(ToolState::Export))
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(new_tool) = new_tool {
                new_tool.unwrap();
                break;
            }
        }
    } else if cancel_just_pressed(&sc_input, &i_mouse_button) && tool_current != ToolState::None {
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }
}

fn cancel_just_pressed(
    scan_code_input: &Res<ScanCodeInput>,
    i_mouse_button: &Res<Input<MouseButton>>,
) -> bool {
    scan_code_input.just_pressed(ScanCode::Escape)
        || i_mouse_button.just_pressed(MouseButton::Right)
}