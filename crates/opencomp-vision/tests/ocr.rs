use opencomp_vision::read;

#[test]
fn read_finds_a_known_word() {
    let png = include_bytes!("fixtures/hello.png");
    let image = image::load_from_memory(png).unwrap();
    let blocks = read(png).unwrap();
    let found: String = blocks
        .iter()
        .flat_map(|block| block.text.chars())
        .filter(|character| character.is_ascii_alphanumeric())
        .collect();

    assert!(
        found.to_ascii_uppercase().contains("HELLO"),
        "blocks were {blocks:?}"
    );
    let width = image.width();
    let height = image.height();
    assert!(blocks.iter().all(|block| {
        block.x < width
            && block.y < height
            && block.width > 0
            && block.height > 0
            && block.x + block.width <= width
            && block.y + block.height <= height
    }));
}
