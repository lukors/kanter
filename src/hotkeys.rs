use bevy::{app::AppExit, prelude::*, window::WindowFocused};

use crate::{
    camera::FirstPersonState,
    delete_tool::DeleteSelected,
    scan_code_input::{ScanCode, ScanCodeInput},
    undo::{
        prelude::{Checkpoint, UndoCommandManager},
        undo_command_manager::UndoCancel,
    },
    CustomStage, GrabToolType, ToolState,
};

pub(crate) struct HotkeysPlugin;

impl Plugin for HotkeysPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set_to_stage(
            CoreStage::Update,
            SystemSet::new()
                .label(CustomStage::Input)
                .with_system(focus_change.system().chain(hotkeys.system())), // .with_system(print_pressed_keys.system())
        )
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new().with_system(quit_hotkey.system()),
        );
    }
}

fn focus_change(
    mut er_window_focused: EventReader<WindowFocused>,
    mut scan_code_input: ResMut<ScanCodeInput>,
) {
    if er_window_focused.iter().any(|event| !event.focused) {
        scan_code_input.clear();
    }
}

fn quit_hotkey(input: Res<ScanCodeInput>, mut app_exit_events: EventWriter<AppExit>) {
    if control_pressed(&input) && input.just_pressed(ScanCode::KeyQ) {
        app_exit_events.send(AppExit);
    }
}

pub(crate) fn control_pressed(scan_code_input: &ScanCodeInput) -> bool {
    scan_code_input.pressed(ScanCode::ControlLeft)
        || scan_code_input.pressed(ScanCode::ControlRight)
}
fn shift_pressed(scan_code_input: &ScanCodeInput) -> bool {
    scan_code_input.pressed(ScanCode::ShiftLeft) || scan_code_input.pressed(ScanCode::ShiftRight)
}
fn alt_pressed(scan_code_input: &ScanCodeInput) -> bool {
    scan_code_input.pressed(ScanCode::AltLeft) || scan_code_input.pressed(ScanCode::AltRight)
}

fn cancel_just_pressed(
    scan_code_input: &ScanCodeInput,
    i_mouse_button: &Input<MouseButton>,
) -> bool {
    scan_code_input.just_pressed(ScanCode::Escape)
        || i_mouse_button.just_pressed(MouseButton::Right)
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
    mut sc_input: ResMut<ScanCodeInput>,
    mut undo_command_manager: ResMut<UndoCommandManager>,
) {
    if sc_input.just_pressed(ScanCode::Backquote) {
        if *first_person_state.current() == FirstPersonState::Off {
            first_person_state.set(FirstPersonState::On).unwrap();
        } else {
            first_person_state.set(FirstPersonState::Off).unwrap();
        }
    }

    let tool_current = tool_state.current().clone();

    let mut just_released_scan_code = Vec::new();
    if tool_current == ToolState::None {
        for scan_code in sc_input.get_just_pressed() {
            let new_tool = match scan_code {
                // ScanCode::Delete | ScanCode::KeyX => Some(tool_state.set(ToolState::Delete)),
                ScanCode::Delete | ScanCode::KeyX => {
                    undo_command_manager.push(Box::new(DeleteSelected));
                    undo_command_manager.push(Box::new(Checkpoint));
                    None
                }
                ScanCode::F12 => Some(tool_state.set(ToolState::Process)),
                ScanCode::KeyA => {
                    if shift_pressed(&sc_input) {
                        Some(tool_state.set(ToolState::Add))
                    } else {
                        None
                    }
                }
                ScanCode::KeyE => {
                    if control_pressed(&sc_input) {
                        if shift_pressed(&sc_input) {
                            Some(tool_state.set(ToolState::ExportOutputs(true)))
                        } else {
                            Some(tool_state.set(ToolState::ExportOutputs(false)))
                        }
                    } else if alt_pressed(&sc_input) && shift_pressed(&sc_input) {
                        Some(tool_state.set(ToolState::Export))
                    } else {
                        None
                    }
                }
                ScanCode::KeyG => Some(tool_state.set(ToolState::Grab(GrabToolType::Node))),
                ScanCode::Tab => Some(tool_state.set(ToolState::EditNode)),
                ScanCode::KeyZ => {
                    if control_pressed(&sc_input) {
                        if shift_pressed(&sc_input) {
                            Some(tool_state.set(ToolState::Redo))
                        } else {
                            Some(tool_state.set(ToolState::Undo))
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(new_tool) = new_tool {
                just_released_scan_code.push(*scan_code);
                new_tool.unwrap();
                break;
            }
        }
    } else if cancel_just_pressed(&sc_input, &i_mouse_button) && tool_current != ToolState::None {
        undo_command_manager.push(Box::new(UndoCancel));
        tool_state.overwrite_replace(ToolState::None).unwrap();
    }

    for scan_code in just_released_scan_code {
        sc_input.clear_just_pressed(scan_code);
    }
}
