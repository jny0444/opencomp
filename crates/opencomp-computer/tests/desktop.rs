use opencomp_computer::Desktop;
use opencomp_core::action::{Action, Point};
use opencomp_core::computer::Computer;

#[tokio::test]
#[ignore]
async fn screenshot_is_a_png_and_move_near_origin() {
    let mut desktop = Desktop::new().unwrap();

    let observation = desktop.screenshot().await.unwrap();
    assert!(!observation.png.is_empty());
    assert!(observation.png.starts_with(&[0x89, b'P', b'N', b'G']));
    assert!(observation.width > 0);
    assert!(observation.height > 0);

    desktop
        .act(&Action::Move(Point { x: 1, y: 1 }))
        .await
        .unwrap();
}
