use image::ImageEncoder;
use opencomp_core::{action::Action, computer::Computer, error::OpenCompCoreError};
use xcap::Monitor;

use crate::input;

const MODEL_MAX_EDGE: u32 = 1280;
const CHANGE_SAMPLE: u32 = 64;

struct FrameCache {
    sample: Vec<u8>,
    png: Vec<u8>,
    width: u32,
    height: u32,
    screen_width: u32,
    screen_height: u32,
}

pub struct Desktop {
    enigo: enigo::Enigo,
    scale_factor: f32,
    monitor: Option<Monitor>,
    frame: Option<FrameCache>,
}

impl Desktop {
    pub fn new() -> Result<Self, OpenCompCoreError> {
        let enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        Ok(Self {
            enigo,
            scale_factor: 0.0,
            monitor: None,
            frame: None,
        })
    }
}

impl Computer for Desktop {
    async fn act(
        &mut self,
        action: &opencomp_core::action::Action,
    ) -> Result<(), OpenCompCoreError> {
        match action {
            Action::Click { point, button } => input::click(
                &mut self.enigo,
                point.clone(),
                button.clone(),
                self.scale_factor,
            ),
            Action::DoubleClick(point) => {
                input::double_click(&mut self.enigo, point.clone(), self.scale_factor)
            }
            Action::Move(point) => {
                input::move_to(&mut self.enigo, point.clone(), self.scale_factor)
            }
            Action::Drag { from, to } => {
                input::drag(&mut self.enigo, from.clone(), to.clone(), self.scale_factor)
            }
            Action::Release { point, button } => input::release(
                &mut self.enigo,
                point.clone(),
                button.clone(),
                self.scale_factor,
            ),
            Action::Type(text) => input::type_text(&mut self.enigo, text),
            Action::Key { keys } => input::press_keys(&mut self.enigo, keys),
            Action::Scroll { dx, dy } => input::scroll(&mut self.enigo, *dx, *dy),
            Action::Wait { millis } => {
                input::wait(*millis);
                Ok(())
            }
            Action::Done { .. } => Err(OpenCompCoreError::InvalidAction(
                "done is not a computer action".to_owned(),
            )),
        }
    }

    async fn screenshot(
        &mut self,
    ) -> Result<opencomp_core::observation::Observation, OpenCompCoreError> {
        if self.monitor.is_none() {
            let monitors =
                Monitor::all().map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;
            let monitor = monitors
                .into_iter()
                .find(|m| m.is_primary().unwrap_or(false))
                .ok_or_else(|| OpenCompCoreError::Computer("no primary monitor".to_string()))?;
            self.monitor = Some(monitor);
        }
        let monitor = self
            .monitor
            .as_ref()
            .ok_or_else(|| OpenCompCoreError::Computer("no primary monitor".to_string()))?;

        let image = monitor
            .capture_image()
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        self.scale_factor = monitor
            .scale_factor()
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        let screen_width = image.width();
        let screen_height = image.height();
        let sample = change_sample(&image);
        if let Some(frame) = &self.frame
            && frame.sample == sample
                && frame.screen_width == screen_width
                && frame.screen_height == screen_height
            {
                return Ok(opencomp_core::observation::Observation {
                    png: frame.png.clone(),
                    width: frame.width,
                    height: frame.height,
                    screen_width,
                    screen_height,
                });
            }

        let (width, height) = model_size(screen_width, screen_height);
        let model_image = if width == screen_width && height == screen_height {
            image
        } else {
            downsample(&image, width, height)
        };
        let png = encode_png(&model_image)
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        self.frame = Some(FrameCache {
            sample,
            png: png.clone(),
            width,
            height,
            screen_width,
            screen_height,
        });

        Ok(opencomp_core::observation::Observation {
            png,
            width,
            height,
            screen_width,
            screen_height,
        })
    }
}

fn model_size(width: u32, height: u32) -> (u32, u32) {
    let long = width.max(height);
    if width == 0 || height == 0 || long <= MODEL_MAX_EDGE {
        return (width, height);
    }
    let width = (u64::from(width) * u64::from(MODEL_MAX_EDGE) / u64::from(long)).max(1) as u32;
    let height = (u64::from(height) * u64::from(MODEL_MAX_EDGE) / u64::from(long)).max(1) as u32;
    (width, height)
}

fn downsample(src: &image::RgbaImage, dst_width: u32, dst_height: u32) -> image::RgbaImage {
    let src_width = src.width();
    let src_height = src.height();
    let src_raw = src.as_raw();
    let mut dst = vec![0u8; dst_width as usize * dst_height as usize * 4];
    for y in 0..dst_height {
        let src_y = (u64::from(y) * u64::from(src_height) / u64::from(dst_height)) as u32;
        let src_row = src_y as usize * src_width as usize * 4;
        let dst_row = y as usize * dst_width as usize * 4;
        for x in 0..dst_width {
            let src_x = (u64::from(x) * u64::from(src_width) / u64::from(dst_width)) as u32;
            let from = src_row + src_x as usize * 4;
            let to = dst_row + x as usize * 4;
            dst[to..to + 4].copy_from_slice(&src_raw[from..from + 4]);
        }
    }
    image::RgbaImage::from_raw(dst_width, dst_height, dst).expect("downsample buffer length")
}

fn change_sample(src: &image::RgbaImage) -> Vec<u8> {
    if src.width() == 0 || src.height() == 0 {
        return Vec::new();
    }
    downsample(src, CHANGE_SAMPLE, CHANGE_SAMPLE).into_raw()
}

fn encode_png(image: &image::RgbaImage) -> Result<Vec<u8>, image::ImageError> {
    let raw = image.as_raw();
    let mut rgb = Vec::with_capacity(raw.len() / 4 * 3);
    for pixel in raw.as_chunks::<4>().0 {
        rgb.extend_from_slice(&pixel[..3]);
    }
    let mut png = std::io::Cursor::new(Vec::new());
    image::codecs::png::PngEncoder::new_with_quality(
        &mut png,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Sub,
    )
    .write_image(
        &rgb,
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(png.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{downsample, model_size};

    #[test]
    fn model_size_caps_the_long_edge_at_1280() {
        assert_eq!(model_size(100, 50), (100, 50));
        assert_eq!(model_size(3024, 1964), (1280, 831));
        assert_eq!(model_size(1000, 4000), (320, 1280));
    }

    #[test]
    fn downsample_picks_nearest_source_pixels() {
        let mut src = image::RgbaImage::new(4, 2);
        src.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        src.put_pixel(3, 0, image::Rgba([0, 0, 255, 255]));
        src.put_pixel(0, 1, image::Rgba([0, 255, 0, 255]));

        let dst = downsample(&src, 2, 1);
        assert_eq!(dst.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(dst.get_pixel(1, 0).0, [0, 0, 0, 0]);
    }

    #[test]
    fn encode_png_is_a_png_smaller_than_the_raw_frame() {
        let (width, height) = model_size(3024, 1964);
        let mut image = image::RgbaImage::new(width, height);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = image::Rgba([(x % 64) as u8, (y % 64) as u8, 40, 255]);
        }
        let png = super::encode_png(&image).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(
            png.len() < (width as usize) * (height as usize),
            "png was {} bytes",
            png.len()
        );
    }
}
