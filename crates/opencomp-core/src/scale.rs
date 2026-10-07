use crate::action::{Action, Point};

pub fn to_screen(
    point: Point,
    model_width: u32,
    model_height: u32,
    screen_width: u32,
    screen_height: u32,
) -> Point {
    if model_width == 0 || model_height == 0 {
        return point;
    }
    if model_width == screen_width && model_height == screen_height {
        return point;
    }
    Point {
        x: scale_axis(point.x, model_width, screen_width),
        y: scale_axis(point.y, model_height, screen_height),
    }
}

pub fn scale_action(
    action: &Action,
    model_width: u32,
    model_height: u32,
    screen_width: u32,
    screen_height: u32,
) -> Action {
    let scale = |point: &Point| {
        to_screen(
            point.clone(),
            model_width,
            model_height,
            screen_width,
            screen_height,
        )
    };
    match action {
        Action::Click { point, button } => Action::Click {
            point: scale(point),
            button: button.clone(),
        },
        Action::DoubleClick(point) => Action::DoubleClick(scale(point)),
        Action::Move(point) => Action::Move(scale(point)),
        Action::Drag { from, to } => Action::Drag {
            from: scale(from),
            to: scale(to),
        },
        Action::Release { point, button } => Action::Release {
            point: scale(point),
            button: button.clone(),
        },
        other => other.clone(),
    }
}

fn scale_axis(value: u32, model: u32, screen: u32) -> u32 {
    if screen == 0 {
        return 0;
    }
    if value >= model.saturating_sub(1) {
        return screen - 1;
    }
    let mapped = u64::from(value) * u64::from(screen) / u64::from(model);
    mapped.min(u64::from(screen - 1)) as u32
}
