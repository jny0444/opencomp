use opencomp_core::action::Key;

pub fn to_enigo(key: opencomp_core::action::Key) -> enigo::Key {
    match key {
        Key::Enter => enigo::Key::Return,
        Key::Super => enigo::Key::Meta,
        Key::Char(c) => enigo::Key::Unicode(c),
        Key::Alt => enigo::Key::Alt,
        Key::Backspace => enigo::Key::Backspace,
        Key::Control => enigo::Key::Control,
        Key::Delete => enigo::Key::Delete,
        Key::Escape => enigo::Key::Escape,
        Key::Shift => enigo::Key::Shift,
        Key::Tab => enigo::Key::Tab,
    }
}
