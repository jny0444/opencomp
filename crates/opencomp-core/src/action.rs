use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Super,
    Alt,
    Control,
    Shift,
    Char(char),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Click { point: Point, button: MouseButton },

    DoubleClick(Point),

    Move(Point),

    Drag { from: Point, to: Point },

    Release { point: Point, button: MouseButton },

    Type(String),

    Key { keys: Vec<Key> },

    Scroll { dx: i32, dy: i32 },

    Wait { millis: u64 },

    Done { result: String },
}
