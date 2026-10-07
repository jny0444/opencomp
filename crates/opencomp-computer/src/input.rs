use std::time::Duration;

use enigo::{Keyboard, Mouse};
use opencomp_core::{
    action::{Key, MouseButton, Point},
    error::OpenCompCoreError,
};

pub(crate) fn click(
    enigo: &mut enigo::Enigo,
    point: Point,
    button: MouseButton,
    scale_factor: f32,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, scale_factor);
    let enigo_button = to_button(button);

    enigo
        .move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    enigo
        .button(enigo_button, enigo::Direction::Click)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    Ok(())
}

pub(crate) fn double_click(
    enigo: &mut enigo::Enigo,
    point: Point,
    scale_factor: f32,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, scale_factor);
    let enigo_button = enigo::Button::Left;

    enigo
        .move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    enigo
        .button(enigo_button, enigo::Direction::Click)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
    enigo
        .button(enigo_button, enigo::Direction::Click)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    Ok(())
}

pub(crate) fn move_to(
    enigo: &mut enigo::Enigo,
    point: Point,
    scale_factor: f32,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, scale_factor);

    enigo
        .move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    Ok(())
}

pub(crate) fn drag(
    enigo: &mut enigo::Enigo,
    from: Point,
    to: Point,
    scale_factor: f32,
) -> Result<(), OpenCompCoreError> {
    let (from_x, from_y) = to_point(from, scale_factor);
    let (to_x, to_y) = to_point(to, scale_factor);

    enigo
        .move_mouse(from_x, from_y, enigo::Coordinate::Abs)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    enigo
        .button(enigo::Button::Left, enigo::Direction::Press)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    enigo
        .move_mouse(to_x, to_y, enigo::Coordinate::Abs)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    enigo
        .button(enigo::Button::Left, enigo::Direction::Release)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    Ok(())
}

pub(crate) fn release(
    enigo: &mut enigo::Enigo,
    point: Point,
    button: MouseButton,
    scale_factor: f32,
) -> Result<(), OpenCompCoreError> {
    let (x, y) = to_point(point, scale_factor);
    let enigo_button = to_button(button);

    enigo
        .move_mouse(x, y, enigo::Coordinate::Abs)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    enigo
        .button(enigo_button, enigo::Direction::Release)
        .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

    Ok(())
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
            .key(crate::keys::to_enigo(key.clone()), enigo::Direction::Release)
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

fn to_button(button: MouseButton) -> enigo::Button {
    match button {
        MouseButton::Left => enigo::Button::Left,
        MouseButton::Middle => enigo::Button::Middle,
        MouseButton::Right => enigo::Button::Right,
    }
}

fn to_point(point: Point, scale_factor: f32) -> (i32, i32) {
    let scale = if scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    (
        (point.x as f32 / scale).round() as i32,
        (point.y as f32 / scale).round() as i32,
    )
}
