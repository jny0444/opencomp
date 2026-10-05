use opencomp_computer::keys::to_enigo;
use opencomp_core::action::Key;

#[test]
fn maps_super_enter_and_char() {
    assert_eq!(to_enigo(Key::Super), enigo::Key::Meta);
    assert_eq!(to_enigo(Key::Enter), enigo::Key::Return);
    assert_eq!(to_enigo(Key::Char('a')), enigo::Key::Unicode('a'));
}
