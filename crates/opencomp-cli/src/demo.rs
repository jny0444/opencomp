use opencomp_core::action::{Action, MouseButton, Point};

pub fn script() -> Vec<Action> {
    vec![
        Action::Move(Point { x: 10, y: 10 }),
        Action::Click {
            point: Point { x: 10, y: 10 },
            button: MouseButton::Left,
        },
        Action::Done {
            result: "demo finished".to_owned(),
        },
    ]
}
