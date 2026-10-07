use opencomp_core::action::{Action, MouseButton, Point};

/// Points are pixels in the model screenshot. `Agent::run` scales them to the capture.
pub fn script() -> Vec<Action> {
    vec![
        Action::Move(Point { x: 10, y: 10 }),
        Action::Click {
            point: Point { x: 10, y: 10 },
            button: MouseButton::Left,
        },
        Action::Release {
            point: Point { x: 10, y: 10 },
            button: MouseButton::Left,
        },
        Action::Move(Point { x: 100, y: 100 }),
        Action::Done {
            result: "demo finished".to_owned(),
        },
    ]
}
