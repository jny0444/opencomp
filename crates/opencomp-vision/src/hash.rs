const SAMPLE: u32 = 64;

/// Hash of a 64×64 gray sample of a PNG. The same pixels hash the same even
/// when the file bytes differ. Bytes that are not a PNG hash as themselves.
pub fn frame_hash(png: &[u8]) -> u64 {
    match gray_sample(png) {
        Some(sample) => hash_bytes(&sample),
        None => hash_bytes(png),
    }
}

fn gray_sample(png: &[u8]) -> Option<Vec<u8>> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    let image = image::load_from_memory(png).ok()?.into_luma8();
    if image.width() == 0 || image.height() == 0 {
        return None;
    }
    Some(downsample(&image, SAMPLE, SAMPLE))
}

fn downsample(src: &image::GrayImage, dst_width: u32, dst_height: u32) -> Vec<u8> {
    let src_width = src.width();
    let src_height = src.height();
    let src_raw = src.as_raw();
    let mut dst = vec![0u8; dst_width as usize * dst_height as usize];
    for y in 0..dst_height {
        let src_y = (u64::from(y) * u64::from(src_height) / u64::from(dst_height)) as u32;
        let src_row = src_y as usize * src_width as usize;
        let dst_row = y as usize * dst_width as usize;
        for x in 0..dst_width {
            let src_x = (u64::from(x) * u64::from(src_width) / u64::from(dst_width)) as u32;
            dst[dst_row + x as usize] = src_raw[src_row + src_x as usize];
        }
    }
    dst
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
