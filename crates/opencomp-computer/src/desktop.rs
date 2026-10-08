use std::time::Duration;

use image::ImageEncoder;
use opencomp_core::{action::Action, computer::Computer, error::OpenCompCoreError};
use xcap::{Monitor, Window};

use crate::input::{self, Aim};

const MODEL_MAX_EDGE: u32 = 1280;
const CHANGE_SAMPLE: u32 = 64;
const MIN_WINDOW: u32 = 80;
const TOOLBAR_REVEAL: Duration = Duration::from_millis(800);

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
    origin_x: i32,
    origin_y: i32,
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
            origin_x: 0,
            origin_y: 0,
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
        let aim = self.aim();
        match action {
            Action::Click { point, button } => {
                input::click(&mut self.enigo, point.clone(), button.clone(), aim)
            }
            Action::DoubleClick(point) => input::double_click(&mut self.enigo, point.clone(), aim),
            Action::Move(point) => input::move_to(&mut self.enigo, point.clone(), aim),
            Action::Drag { from, to } => {
                input::drag(&mut self.enigo, from.clone(), to.clone(), aim)
            }
            Action::Release { point, button } => {
                input::release(&mut self.enigo, point.clone(), button.clone(), aim)
            }
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
        let shot = self.capture()?;
        self.observe(shot)
    }
}

impl Desktop {
    pub async fn screenshot_primary(
        &mut self,
    ) -> Result<opencomp_core::observation::Observation, OpenCompCoreError> {
        let shot = self.capture_primary()?;
        self.observe(shot)
    }

    fn observe(
        &mut self,
        shot: Shot,
    ) -> Result<opencomp_core::observation::Observation, OpenCompCoreError> {
        self.scale_factor = shot.scale_factor;
        self.origin_x = shot.origin_x;
        self.origin_y = shot.origin_y;

        let screen_width = shot.image.width();
        let screen_height = shot.image.height();
        let sample = change_sample(&shot.image);
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
            shot.image
        } else {
            downsample(&shot.image, width, height)
        };
        let png =
            encode_png(&model_image).map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

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

impl Desktop {
    fn aim(&self) -> Aim {
        Aim {
            scale_factor: self.scale_factor,
            origin_x: self.origin_x,
            origin_y: self.origin_y,
        }
    }

    fn capture(&mut self) -> Result<Shot, OpenCompCoreError> {
        let mut windows = list_windows()?;
        let mut geoms: Vec<Win> = windows.iter().map(|window| window.geom.clone()).collect();
        if let Some(content_at) = content_index(&geoms)
            && title_strip_index(&geoms, content_at).is_some_and(|strip_at| geoms[strip_at].y < 0)
        {
            let content = &geoms[content_at];
            let x = content.x + content.width as i32 / 2;
            let y = content.y.max(0);
            match input::move_screen(&mut self.enigo, x, y) {
                Ok(()) => {
                    tracing::info!(x, y, "revealing the frontmost window toolbar");
                    std::thread::sleep(TOOLBAR_REVEAL);
                    windows = list_windows()?;
                    geoms = windows.iter().map(|window| window.geom.clone()).collect();
                }
                Err(error) => {
                    tracing::warn!(error = %error, "could not reveal the window toolbar");
                }
            }
        }

        if let Some(content_at) = content_index(&geoms) {
            if let Some(strip_at) = title_strip_index(&geoms, content_at)
                && geoms[strip_at].y >= 0
                && let Some(shot) = capture_union(&windows[content_at], &windows[strip_at])?
            {
                return Ok(shot);
            }
            return capture_one(&windows[content_at]);
        }

        self.capture_primary()
    }

    fn capture_primary(&mut self) -> Result<Shot, OpenCompCoreError> {
        if self.monitor.is_none() {
            let monitors =
                Monitor::all().map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
            let monitor = monitors
                .into_iter()
                .find(|monitor| monitor.is_primary().unwrap_or(false))
                .ok_or_else(|| OpenCompCoreError::Computer("no primary monitor".to_string()))?;
            self.monitor = Some(monitor);
        }
        let monitor = self
            .monitor
            .as_ref()
            .ok_or_else(|| OpenCompCoreError::Computer("no primary monitor".to_string()))?;
        let image = monitor
            .capture_image()
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let scale_factor = monitor
            .scale_factor()
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let origin_x = monitor
            .x()
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let origin_y = monitor
            .y()
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        tracing::info!(origin_x, origin_y, "capturing primary monitor");
        Ok(Shot {
            image,
            scale_factor,
            origin_x,
            origin_y,
        })
    }
}

struct Shot {
    image: image::RgbaImage,
    scale_factor: f32,
    origin_x: i32,
    origin_y: i32,
}

struct Listed {
    window: Window,
    geom: Win,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Win {
    app: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    focused: bool,
    minimized: bool,
}

fn list_windows() -> Result<Vec<Listed>, OpenCompCoreError> {
    let windows = Window::all().map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let mut listed = Vec::new();
    for window in windows {
        let Some(geom) = win_geom(&window) else {
            continue;
        };
        listed.push(Listed { window, geom });
    }
    Ok(listed)
}

fn win_geom(window: &Window) -> Option<Win> {
    Some(Win {
        app: window.app_name().ok()?,
        x: window.x().ok()?,
        y: window.y().ok()?,
        width: window.width().ok()?,
        height: window.height().ok()?,
        focused: window.is_focused().unwrap_or(false),
        minimized: window.is_minimized().unwrap_or(true),
    })
}

fn content_index(windows: &[Win]) -> Option<usize> {
    windows.iter().position(|window| {
        window.focused
            && !window.minimized
            && window.width >= MIN_WINDOW
            && window.height >= MIN_WINDOW
    })
}

fn title_strip_index(windows: &[Win], content_at: usize) -> Option<usize> {
    let content = windows.get(content_at)?.clone();
    let content_right = content.x.saturating_add(content.width as i32);
    windows.iter().position(|window| {
        if window.app != content.app || window.minimized {
            return false;
        }
        if window.height == 0 || window.height >= MIN_WINDOW {
            return false;
        }
        let bottom = window.y.saturating_add(window.height as i32);
        let right = window.x.saturating_add(window.width as i32);
        let overlaps_x =
            window.x <= content.x.saturating_add(40) && right >= content_right.saturating_sub(40);
        let overlaps_top =
            window.y < content.y.saturating_add(80) && bottom >= content.y.saturating_sub(4);
        overlaps_x && overlaps_top
    })
}

fn union_rect(content: &Win, strip: &Win) -> (i32, i32, u32, u32) {
    let x = content.x.min(strip.x);
    let y = content.y.min(strip.y);
    let right = content
        .x
        .saturating_add(content.width as i32)
        .max(strip.x.saturating_add(strip.width as i32));
    let bottom = content
        .y
        .saturating_add(content.height as i32)
        .max(strip.y.saturating_add(strip.height as i32));
    let width = right.saturating_sub(x).max(0) as u32;
    let height = bottom.saturating_sub(y).max(0) as u32;
    (x, y, width, height)
}

fn clamp_to_monitor(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitor_x: i32,
    monitor_y: i32,
    monitor_width: u32,
    monitor_height: u32,
) -> Option<(u32, u32, u32, u32, i32, i32)> {
    let right = x.saturating_add(width as i32);
    let bottom = y.saturating_add(height as i32);
    let monitor_right = monitor_x.saturating_add(monitor_width as i32);
    let monitor_bottom = monitor_y.saturating_add(monitor_height as i32);
    let left = x.max(monitor_x);
    let top = y.max(monitor_y);
    let right = right.min(monitor_right);
    let bottom = bottom.min(monitor_bottom);
    if right <= left || bottom <= top {
        return None;
    }
    Some((
        (left - monitor_x) as u32,
        (top - monitor_y) as u32,
        (right - left) as u32,
        (bottom - top) as u32,
        left,
        top,
    ))
}

fn capture_one(window: &Listed) -> Result<Shot, OpenCompCoreError> {
    let image = window
        .window
        .capture_image()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let monitor = window
        .window
        .current_monitor()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let scale_factor = monitor
        .scale_factor()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let title = window.window.title().unwrap_or_default();
    tracing::info!(
        title = %title,
        origin_x = window.geom.x,
        origin_y = window.geom.y,
        pixels_wide = image.width(),
        pixels_high = image.height(),
        "capturing frontmost window"
    );
    Ok(Shot {
        image,
        scale_factor,
        origin_x: window.geom.x,
        origin_y: window.geom.y,
    })
}

fn capture_union(content: &Listed, strip: &Listed) -> Result<Option<Shot>, OpenCompCoreError> {
    let (x, y, width, height) = union_rect(&content.geom, &strip.geom);
    let monitor = content
        .window
        .current_monitor()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let scale_factor = monitor
        .scale_factor()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let monitor_x = monitor
        .x()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let monitor_y = monitor
        .y()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let monitor_width = monitor
        .width()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let monitor_height = monitor
        .height()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    let Some((region_x, region_y, width, height, origin_x, origin_y)) = clamp_to_monitor(
        x,
        y,
        width,
        height,
        monitor_x,
        monitor_y,
        monitor_width,
        monitor_height,
    ) else {
        return Ok(None);
    };
    let image = monitor
        .capture_region(region_x, region_y, width, height)
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    tracing::info!(
        origin_x,
        origin_y,
        pixels_wide = image.width(),
        pixels_high = image.height(),
        "capturing frontmost window and toolbar"
    );
    Ok(Some(Shot {
        image,
        scale_factor,
        origin_x,
        origin_y,
    }))
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
    use opencomp_core::computer::Computer;

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

    fn preview(x: i32, y: i32, width: u32, height: u32) -> super::Win {
        super::Win {
            app: "Preview".to_owned(),
            x,
            y,
            width,
            height,
            focused: true,
            minimized: false,
        }
    }

    #[test]
    fn a_fullscreen_title_strip_is_not_the_content_window() {
        let windows = vec![preview(0, -52, 1440, 52), preview(0, 0, 1440, 900)];
        let content = super::content_index(&windows).unwrap();
        assert_eq!(content, 1);
        let strip = super::title_strip_index(&windows, content).unwrap();
        assert_eq!(windows[strip].y, -52);
        let (x, y, width, height) = super::union_rect(&windows[content], &windows[strip]);
        assert_eq!((x, y, width, height), (0, -52, 1440, 952));
        let clamped = super::clamp_to_monitor(x, y, width, height, 0, 0, 1440, 900).unwrap();
        assert_eq!(clamped, (0, 0, 1440, 900, 0, 0));
    }

    #[test]
    fn a_revealed_fullscreen_toolbar_overlaps_the_document() {
        let windows = vec![preview(0, 30, 1440, 52), preview(0, 0, 1440, 900)];
        let content = super::content_index(&windows).unwrap();
        let strip = super::title_strip_index(&windows, content).unwrap();
        assert_eq!(windows[strip].y, 30);
    }

    #[test]
    fn an_on_screen_toolbar_stays_in_the_capture_rect() {
        let content = preview(200, 80, 800, 600);
        let strip = preview(200, 40, 800, 40);
        let (x, y, width, height) = super::union_rect(&content, &strip);
        let clamped = super::clamp_to_monitor(x, y, width, height, 0, 0, 1440, 900).unwrap();
        assert_eq!(clamped, (200, 40, 800, 640, 200, 40));
    }

    #[tokio::test]
    #[ignore]
    async fn frontmost_window_capture_reports_an_origin() {
        let mut desktop = super::Desktop::new().unwrap();
        super::input::move_screen(&mut desktop.enigo, 720, 450).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let observation = desktop.screenshot().await.unwrap();
        let path = std::env::temp_dir().join("opencomp-frontmost.png");
        std::fs::write(&path, &observation.png).unwrap();
        let image = image::load_from_memory(&observation.png).unwrap().to_rgb8();
        let mut red = false;
        let x_end = 80.min(image.width());
        let y_end = 100.min(image.height());
        for y in 0..y_end {
            for x in 0..x_end {
                let pixel = image.get_pixel(x, y);
                if pixel[0] > 180 && pixel[1] < 140 && pixel[2] < 140 {
                    red = true;
                }
            }
        }
        eprintln!(
            "model {}x{} capture {}x{} origin {},{} scale {} red={} png {}",
            observation.width,
            observation.height,
            observation.screen_width,
            observation.screen_height,
            desktop.origin_x,
            desktop.origin_y,
            desktop.scale_factor,
            red,
            path.display()
        );
        assert!(red, "close button was not in the frontmost capture");
    }
}
