use opencomp_core::{
    action::{Action, Key, Point},
    error::OpenCompCoreError,
};

pub fn check(action: &Action, width: u32, height: u32) -> Result<(), OpenCompCoreError> {
    match action {
        Action::Click { point, .. } | Action::DoubleClick(point) | Action::Move(point) => {
            check_point(point, width, height)
        }
        Action::Drag { from, to } => {
            check_point(from, width, height)?;
            check_point(to, width, height)
        }
        Action::Release { point, .. } => check_point(point, width, height),
        Action::Key { keys } => check_keys(keys),
        Action::Type(_) | Action::Scroll { .. } | Action::Wait { .. } | Action::Done { .. } => {
            Ok(())
        }
    }
}

fn check_point(point: &Point, width: u32, height: u32) -> Result<(), OpenCompCoreError> {
    if point.x >= width || point.y >= height {
        return Err(OpenCompCoreError::InvalidAction(format!(
            "point {},{} is outside {width}x{height}",
            point.x, point.y
        )));
    }
    Ok(())
}

fn check_keys(keys: &[Key]) -> Result<(), OpenCompCoreError> {
    let has_super = keys.iter().any(|key| matches!(key, Key::Super));
    let quits = keys.iter().any(|key| matches!(key, Key::Char('q' | 'Q')));
    if has_super && quits {
        return Err(OpenCompCoreError::InvalidAction(
            "super+q is not allowed".to_owned(),
        ));
    }
    if keys.len() == 1 || is_super_shortcut(keys) {
        return Ok(());
    }
    Err(OpenCompCoreError::InvalidAction(
        "key chord is not allowed".to_owned(),
    ))
}

fn is_super_shortcut(keys: &[Key]) -> bool {
    if keys.len() != 2 || !keys.iter().any(|key| matches!(key, Key::Super)) {
        return false;
    }
    keys.iter()
        .any(|key| matches!(key, Key::Char('c' | 'v' | 'a' | 't' | 'w') | Key::Tab))
}
