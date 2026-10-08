use std::time::Duration;

use enigo::{Keyboard, Mouse};
use opencomp_core::{
    action::{Key, MouseButton, Point},
    error::OpenCompCoreError,
};

#[derive(Clone, Copy)]
pub(crate) struct Aim {
    pub scale_factor: f32,
    pub origin_x: i32,
    pub origin_y: i32,
}

pub(crate) fn click(
    enigo: &mut enigo::Enigo,
    point: Point,
    button: MouseButton,
    aim: Aim,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, aim);
    crate::pointer::click(enigo, x, y, to_pointer_button(button))
}

pub(crate) fn double_click(
    enigo: &mut enigo::Enigo,
    point: Point,
    aim: Aim,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, aim);
    crate::pointer::double_click(enigo, x, y)
}

pub(crate) fn move_to(
    enigo: &mut enigo::Enigo,
    point: Point,
    aim: Aim,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, aim);
    crate::pointer::move_to(enigo, x, y)
}

pub(crate) fn move_screen(
    enigo: &mut enigo::Enigo,
    x: i32,
    y: i32,
) -> Result<(), OpenCompCoreError> {
    crate::pointer::move_to(enigo, x, y)
}

pub(crate) fn drag(
    enigo: &mut enigo::Enigo,
    from: Point,
    to: Point,
    aim: Aim,
) -> Result<(), OpenCompCoreError> {
    let (from_x, from_y) = to_point(from, aim);
    let (to_x, to_y) = to_point(to, aim);
    crate::pointer::drag(enigo, from_x, from_y, to_x, to_y)
}

pub(crate) fn release(
    enigo: &mut enigo::Enigo,
    point: Point,
    button: MouseButton,
    aim: Aim,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, aim);
    crate::pointer::release(enigo, x, y, to_pointer_button(button))
}

pub(crate) fn type_text(enigo: &mut enigo::Enigo, text: &str) -> Result<(), OpenCompCoreError> {
    enigo
        .text(text)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    Ok(())
}

pub(crate) fn press_keys(enigo: &mut enigo::Enigo, keys: &[Key]) -> Result<(), OpenCompCoreError> {
    for key in keys {
        enigo
            .key(crate::keys::to_enigo(key.clone()), enigo::Direction::Press)
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
    }
    for key in keys.iter().rev() {
        enigo
            .key(
                crate::keys::to_enigo(key.clone()),
                enigo::Direction::Release,
            )
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
    }

    Ok(())
}

pub(crate) fn scroll(enigo: &mut enigo::Enigo, dx: i32, dy: i32) -> Result<(), OpenCompCoreError> {
    if dy != 0 {
        enigo
            .scroll(dy, enigo::Axis::Vertical)
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
    }
    if dx != 0 {
        enigo
            .scroll(dx, enigo::Axis::Horizontal)
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
    }

    Ok(())
}

pub(crate) fn wait(millis: u64) {
    std::thread::sleep(Duration::from_millis(millis));
}

fn to_pointer_button(button: MouseButton) -> crate::pointer::Button {
    match button {
        MouseButton::Left => crate::pointer::Button::Left,
        MouseButton::Middle => crate::pointer::Button::Middle,
        MouseButton::Right => crate::pointer::Button::Right,
    }
}

fn to_point(point: Point, aim: Aim) -> (i32, i32) {
    let scale = if aim.scale_factor > 0.0 {
        aim.scale_factor
    } else {
        1.0
    };
    (
        (point.x as f32 / scale).round() as i32 + aim.origin_x,
        (point.y as f32 / scale).round() as i32 + aim.origin_y,
    )
}

#[cfg(test)]
mod tests {
    use opencomp_core::action::Point;

    use super::{Aim, to_point};

    #[test]
    fn a_window_origin_is_added_in_screen_points() {
        let point = to_point(
            Point { x: 28, y: 40 },
            Aim {
                scale_factor: 2.0,
                origin_x: 100,
                origin_y: 200,
            },
        );
        assert_eq!(point, (114, 220));
    }
}
