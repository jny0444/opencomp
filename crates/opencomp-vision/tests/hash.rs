use image::ImageEncoder;
use opencomp_vision::frame_hash;

fn png(pixel: [u8; 3], compression: image::codecs::png::CompressionType) -> Vec<u8> {
    let mut image = image::RgbImage::new(32, 16);
    for px in image.pixels_mut() {
        *px = image::Rgb(pixel);
    }
    let mut png = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new_with_quality(
        &mut png,
        compression,
        image::codecs::png::FilterType::Sub,
    )
    .write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    png.into_inner()
}

#[test]
fn the_same_pixels_hash_the_same_when_the_file_bytes_differ() {
    let fast = png([20, 40, 60], image::codecs::png::CompressionType::Fast);
    let best = png([20, 40, 60], image::codecs::png::CompressionType::Best);
    assert_ne!(fast, best);
    assert_eq!(frame_hash(&fast), frame_hash(&best));
}

#[test]
fn a_changed_pixel_changes_the_hash() {
    let before = png([20, 40, 60], image::codecs::png::CompressionType::Fast);
    let after = png([200, 40, 60], image::codecs::png::CompressionType::Fast);
    assert_ne!(frame_hash(&before), frame_hash(&after));
}

#[test]
fn bytes_that_are_not_a_png_hash_as_themselves() {
    assert_eq!(frame_hash(&[0]), frame_hash(&[0]));
    assert_ne!(frame_hash(&[0]), frame_hash(&[1]));
}
