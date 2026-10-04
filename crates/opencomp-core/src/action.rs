pub struct Point {
    pub x: u32,
    pub y: u32,
}

pub enum Action {
    Click(Point),

    DoubleClick(Point),

    Move(Point),

    Type(String),

    KeyPress(Key),

    Scroll { dx: u32, dy: u32 },

    Wait(std::time::Duration),

    Done { result: String },
}
