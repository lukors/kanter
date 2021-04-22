/// This Bevy plugin enables the use of ScanCodes as easily as KeyCodes can be used.
/// ScanCode names are based on https://w3c.github.io/uievents-code/

use bevy::{
    input::{keyboard::KeyboardInput, ElementState},
    prelude::*,
    utils::HashSet,
};
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum ScanCode {
    Escape = 1,
    Digit1 = 2,
    Digit2 = 3,
    Digit3 = 4,
    Digit4 = 5,
    Digit5 = 6,
    Digit6 = 7,
    Digit7 = 8,
    Digit8 = 9,
    Digit9 = 10,
    Digit0 = 11,
    Minus = 12,
    Equal = 13,
    Backspace = 14,
    Tab = 15,
    KeyQ = 16,
    KeyW = 17,
    KeyE = 18,
    KeyR = 19,
    KeyT = 20,
    KeyY = 21,
    KeyU = 22,
    KeyI = 23,
    KeyO = 24,
    KeyP = 25,
    BracketLeft = 26,
    BracketRight = 27,
    Enter = 28,
    ControlLeft = 29,
    KeyA = 30,
    KeyS = 31,
    KeyD = 32,
    KeyF = 33,
    KeyG = 34,
    KeyH = 35,
    KeyJ = 36,
    KeyK = 37,
    KeyL = 38,
    Semicolon = 39,
    Quote = 40,
    Backquote = 41,
    ShiftLeft = 42,
    BackSlash = 43,
    KeyZ = 44,
    KeyX = 45,
    KeyC = 46,
    KeyV = 47,
    KeyB = 48,
    KeyN = 49,
    KeyM = 50,
    Comma = 51,
    Period = 52,
    Slash = 53,
    ShiftRight = 54,
    AltLeft = 56,
    Space = 57,
    CapsLock = 58,
    F1 = 59,
    F2 = 60,
    F3 = 61,
    F4 = 62,
    F5 = 63,
    F6 = 64,
    F7 = 65,
    F8 = 66,
    F9 = 67,
    F10 = 68,
    F11 = 87,
    F12 = 88,
    ControlRight = 97,
    PrintScreen = 99,
    AltRight = 100,
    Home = 102,
    ArrowUp = 103,
    PageUp = 104,
    ArrowLeft = 105,
    ArrowRight = 106,
    End = 107,
    ArrowDown = 108,
    PageDown = 109,
    Delete = 111,
    MetaLeft = 125,
}

pub struct ScanCodeInputPlugin;

impl Plugin for ScanCodeInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ScanCodeInput::default())
            .add_system_to_stage(CoreStage::First, scan_code_grab_input.system());
    }
}

#[derive(Debug, Default)]
pub struct ScanCodeInput {
    pressed: HashSet<ScanCode>,
    just_pressed: HashSet<ScanCode>,
    just_released: HashSet<ScanCode>,
}

impl ScanCodeInput {
    /// Register a press for input `input`.
    pub fn press(&mut self, input: ScanCode) {
        if !self.pressed(input) {
            self.just_pressed.insert(input);
        }

        self.pressed.insert(input);
    }

    /// Check if `input` has been pressed.
    pub fn pressed(&self, input: ScanCode) -> bool {
        self.pressed.contains(&input)
    }

    /// Register a release for input `input`.
    pub fn release(&mut self, input: ScanCode) {
        self.pressed.remove(&input);
        self.just_released.insert(input);
    }

    /// Check if `input` has been just pressed.
    pub fn just_pressed(&self, input: ScanCode) -> bool {
        self.just_pressed.contains(&input)
    }

    /// Clear the "just pressed" state of `input`. Future calls to [`Input::just_pressed`] for the
    /// given input will return false until a new press event occurs.
    /// Returns true if `input` is currently "just pressed"
    pub fn clear_just_pressed(&mut self, input: ScanCode) -> bool {
        self.just_pressed.remove(&input)
    }

    /// Check if `input` has been just released.
    pub fn just_released(&self, input: ScanCode) -> bool {
        self.just_released.contains(&input)
    }

    /// Clear the "just released" state of `input`. Future calls to [`Input::just_released`] for the
    /// given input will return false until a new release event occurs.
    /// Returns true if `input` is currently "just released"
    pub fn clear_just_released(&mut self, input: ScanCode) -> bool {
        self.just_released.remove(&input)
    }

    /// Reset all status for input `input`.
    pub fn reset(&mut self, input: ScanCode) {
        self.pressed.remove(&input);
        self.just_pressed.remove(&input);
        self.just_released.remove(&input);
    }

    /// Clear just pressed and just released information.
    pub fn clear(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    /// List all inputs that are pressed.
    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &ScanCode> {
        self.pressed.iter()
    }

    /// List all inputs that are just pressed.
    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &ScanCode> {
        self.just_pressed.iter()
    }

    /// List all inputs that are just released.
    pub fn get_just_released(&self) -> impl ExactSizeIterator<Item = &ScanCode> {
        self.just_released.iter()
    }
}

/// Updates the input by clearing the list and grabbing any new events.
fn scan_code_grab_input(
    mut keyboard_input: EventReader<KeyboardInput>,
    mut scan_code_input: ResMut<ScanCodeInput>,
) {
    scan_code_input.clear();

    for keyboard_input in keyboard_input.iter() {
        if let Ok(scan_code) = ScanCode::try_from(keyboard_input.scan_code) {
            match keyboard_input.state {
                ElementState::Pressed => scan_code_input.press(scan_code),
                ElementState::Released => scan_code_input.release(scan_code),
            }
        } else {
            error!("ScanCode has not been implemented: {:?}", keyboard_input);
        }
    }
}
