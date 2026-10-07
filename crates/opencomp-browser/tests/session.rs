use opencomp_browser::Session;
use opencomp_core::computer::Computer;

#[tokio::test]
#[ignore]
async fn screenshot_is_a_non_empty_png() {
    let mut session = Session::launch().await.unwrap();
    let observation = session.screenshot().await.unwrap();

    assert!(!observation.png.is_empty());
    assert_eq!(&observation.png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(observation.width > 0);
    assert!(observation.height > 0);
    assert_eq!(observation.width, observation.screen_width);
    assert_eq!(observation.height, observation.screen_height);
}
