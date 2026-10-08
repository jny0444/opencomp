#[cfg(not(target_os = "macos"))]
use enigo::Mouse;
use opencomp_core::error::OpenCompCoreError;

pub(crate) fn move_to(enigo: &mut enigo::Enigo, x: i32, y: i32) -> Result<(), OpenCompCoreError> {
    tracing::info!(x, y, "moving pointer");
    post(enigo, Pointer::Move, x, y, Button::Left, 0)
}

pub(crate) fn click(
    enigo: &mut enigo::Enigo,
    x: i32,
    y: i32,
    button: Button,
) -> Result<(), OpenCompCoreError> {
    tracing::info!(x, y, ?button, "clicking");
    move_to(enigo, x, y)?;
    std::thread::sleep(std::time::Duration::from_millis(40));
    post(enigo, Pointer::Down, x, y, button, 1)?;
    post(enigo, Pointer::Up, x, y, button, 1)
}

pub(crate) fn double_click(
    enigo: &mut enigo::Enigo,
    x: i32,
    y: i32,
) -> Result<(), OpenCompCoreError> {
    tracing::info!(x, y, "double clicking");
    move_to(enigo, x, y)?;
    std::thread::sleep(std::time::Duration::from_millis(40));
    post(enigo, Pointer::Down, x, y, Button::Left, 1)?;
    post(enigo, Pointer::Up, x, y, Button::Left, 1)?;
    post(enigo, Pointer::Down, x, y, Button::Left, 2)?;
    post(enigo, Pointer::Up, x, y, Button::Left, 2)
}

pub(crate) fn drag(
    enigo: &mut enigo::Enigo,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> Result<(), OpenCompCoreError> {
    tracing::info!(from_x, from_y, to_x, to_y, "dragging");
    move_to(enigo, from_x, from_y)?;
    post(enigo, Pointer::Down, from_x, from_y, Button::Left, 1)?;
    post(enigo, Pointer::Drag, to_x, to_y, Button::Left, 0)?;
    post(enigo, Pointer::Up, to_x, to_y, Button::Left, 1)
}

pub(crate) fn release(
    enigo: &mut enigo::Enigo,
    x: i32,
    y: i32,
    button: Button,
) -> Result<(), OpenCompCoreError> {
    tracing::info!(x, y, ?button, "releasing pointer");
    post(enigo, Pointer::Up, x, y, button, 1)
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Button {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy)]
enum Pointer {
    Move,
    Drag,
    Down,
    Up,
}

#[cfg(target_os = "macos")]
fn post(
    _enigo: &mut enigo::Enigo,
    kind: Pointer,
    x: i32,
    y: i32,
    button: Button,
    clicks: i64,
) -> Result<(), OpenCompCoreError> {
    use core_graphics::event::{
        CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, EventField,
    };
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use core_graphics::geometry::CGPoint;

    let (cg_button, event_type) = match (kind, button) {
        (Pointer::Move, _) => (CGMouseButton::Left, CGEventType::MouseMoved),
        (Pointer::Drag, _) => (CGMouseButton::Left, CGEventType::LeftMouseDragged),
        (Pointer::Down, Button::Left) => (CGMouseButton::Left, CGEventType::LeftMouseDown),
        (Pointer::Up, Button::Left) => (CGMouseButton::Left, CGEventType::LeftMouseUp),
        (Pointer::Down, Button::Right) => (CGMouseButton::Right, CGEventType::RightMouseDown),
        (Pointer::Up, Button::Right) => (CGMouseButton::Right, CGEventType::RightMouseUp),
        (Pointer::Down, Button::Middle) => (CGMouseButton::Center, CGEventType::OtherMouseDown),
        (Pointer::Up, Button::Middle) => (CGMouseButton::Center, CGEventType::OtherMouseUp),
    };
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).map_err(|_| {
        OpenCompCoreError::Computer("could not create a mouse event source".to_owned())
    })?;
    let event = CGEvent::new_mouse_event(
        source,
        event_type,
        CGPoint::new(x as f64, y as f64),
        cg_button,
    )
    .map_err(|_| OpenCompCoreError::Computer("could not create a mouse event".to_owned()))?;
    if matches!(kind, Pointer::Down | Pointer::Up) {
        event.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, clicks);
    }
    event.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn post(
    enigo: &mut enigo::Enigo,
    kind: Pointer,
    x: i32,
    y: i32,
    button: Button,
    _clicks: i64,
) -> Result<(), OpenCompCoreError> {
    let enigo_button = match button {
        Button::Left => enigo::Button::Left,
        Button::Right => enigo::Button::Right,
        Button::Middle => enigo::Button::Middle,
    };
    match kind {
        Pointer::Move | Pointer::Drag => enigo
            .move_mouse(x, y, enigo::Coordinate::Abs)
            .map_err(|error| OpenCompCoreError::Computer(error.to_string())),
        Pointer::Down => enigo
            .button(enigo_button, enigo::Direction::Press)
            .map_err(|error| OpenCompCoreError::Computer(error.to_string())),
        Pointer::Up => enigo
            .button(enigo_button, enigo::Direction::Release)
            .map_err(|error| OpenCompCoreError::Computer(error.to_string())),
    }
}
