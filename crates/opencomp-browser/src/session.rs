use opencomp_core::{
    action::{Action, Key, MouseButton, Point},
    computer::Computer,
    error::OpenCompCoreError,
    observation::Observation,
};
use playwright_rs::{Browser, BrowserContext, Page, Playwright, Viewport};

const VIEWPORT_WIDTH: u32 = 1280;
const VIEWPORT_HEIGHT: u32 = 720;

pub struct Session {
    page: Page,
    _context: BrowserContext,
    _browser: Browser,
    _playwright: Playwright,
}

impl Session {
    pub async fn launch() -> Result<Self, OpenCompCoreError> {
        let playwright = Playwright::launch()
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let browser = playwright
            .chromium()
            .launch()
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let context = browser
            .new_context_with_options(
                playwright_rs::BrowserContextOptions::builder()
                    .viewport(Viewport {
                        width: VIEWPORT_WIDTH,
                        height: VIEWPORT_HEIGHT,
                    })
                    .build(),
            )
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let page = context
            .new_page()
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        page.goto("about:blank", None)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;

        Ok(Self {
            page,
            _context: context,
            _browser: browser,
            _playwright: playwright,
        })
    }
}

impl Computer for Session {
    async fn screenshot(&mut self) -> Result<Observation, OpenCompCoreError> {
        let png = self
            .page
            .screenshot(None)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let (width, height) = png_size(&png)?;
        let viewport = self.page.viewport_size().ok_or_else(|| {
            OpenCompCoreError::Computer("browser page has no viewport".to_owned())
        })?;
        Ok(Observation {
            png,
            width,
            height,
            screen_width: viewport.width,
            screen_height: viewport.height,
        })
    }

    async fn act(&mut self, action: &Action) -> Result<(), OpenCompCoreError> {
        match action {
            Action::Click { point, button } => self.click(point, button).await,
            Action::DoubleClick(point) => self.double_click(point).await,
            Action::Move(point) => self.move_to(point).await,
            Action::Drag { from, to } => self.drag(from, to).await,
            Action::Release { point, button } => self.release(point, button).await,
            Action::Type(text) => self
                .page
                .keyboard()
                .type_text(text, None)
                .await
                .map_err(|error| OpenCompCoreError::Computer(error.to_string())),
            Action::Key { keys } => self.press_keys(keys).await,
            Action::Scroll { dx, dy } => self
                .page
                .mouse()
                .wheel(f64::from(*dx), f64::from(*dy))
                .await
                .map_err(|error| OpenCompCoreError::Computer(error.to_string())),
            Action::Wait { millis } => {
                tokio::time::sleep(std::time::Duration::from_millis(*millis)).await;
                Ok(())
            }
            Action::Done { .. } => Err(OpenCompCoreError::InvalidAction(
                "done is not a computer action".to_owned(),
            )),
        }
    }
}

impl Session {
    async fn click(&self, point: &Point, button: &MouseButton) -> Result<(), OpenCompCoreError> {
        self.page
            .mouse()
            .click(
                f64::from(point.x),
                f64::from(point.y),
                mouse_options(button),
            )
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))
    }

    async fn double_click(&self, point: &Point) -> Result<(), OpenCompCoreError> {
        self.page
            .mouse()
            .dblclick(
                f64::from(point.x),
                f64::from(point.y),
                mouse_options(&MouseButton::Left),
            )
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))
    }

    async fn move_to(&self, point: &Point) -> Result<(), OpenCompCoreError> {
        self.page
            .mouse()
            .move_to(f64::from(point.x), f64::from(point.y), None)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))
    }

    async fn drag(&self, from: &Point, to: &Point) -> Result<(), OpenCompCoreError> {
        let mouse = self.page.mouse();
        let button = mouse_options(&MouseButton::Left);
        mouse
            .move_to(f64::from(from.x), f64::from(from.y), None)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        mouse
            .down(button.clone())
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        mouse
            .move_to(f64::from(to.x), f64::from(to.y), None)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        mouse
            .up(button)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))
    }

    async fn release(&self, point: &Point, button: &MouseButton) -> Result<(), OpenCompCoreError> {
        let mouse = self.page.mouse();
        mouse
            .move_to(f64::from(point.x), f64::from(point.y), None)
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        mouse
            .up(mouse_options(button))
            .await
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))
    }

    async fn press_keys(&self, keys: &[Key]) -> Result<(), OpenCompCoreError> {
        let keyboard = self.page.keyboard();
        for key in keys {
            keyboard
                .down(&key_name(key))
                .await
                .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        }
        for key in keys.iter().rev() {
            keyboard
                .up(&key_name(key))
                .await
                .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        }
        Ok(())
    }
}

fn mouse_options(button: &MouseButton) -> playwright_rs::protocol::MouseOptions {
    playwright_rs::protocol::MouseOptions::builder()
        .button(match button {
            MouseButton::Left => playwright_rs::protocol::click::MouseButton::Left,
            MouseButton::Right => playwright_rs::protocol::click::MouseButton::Right,
            MouseButton::Middle => playwright_rs::protocol::click::MouseButton::Middle,
        })
        .build()
}

fn key_name(key: &Key) -> String {
    match key {
        Key::Enter => "Enter".to_owned(),
        Key::Escape => "Escape".to_owned(),
        Key::Tab => "Tab".to_owned(),
        Key::Backspace => "Backspace".to_owned(),
        Key::Delete => "Delete".to_owned(),
        Key::Super => "Meta".to_owned(),
        Key::Alt => "Alt".to_owned(),
        Key::Control => "Control".to_owned(),
        Key::Shift => "Shift".to_owned(),
        Key::Char(character) => character.to_string(),
    }
}

fn png_size(png: &[u8]) -> Result<(u32, u32), OpenCompCoreError> {
    if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(OpenCompCoreError::Computer(
            "browser screenshot is not a png".to_owned(),
        ));
    }
    let width = u32::from_be_bytes(png[16..20].try_into().expect("png width bytes"));
    let height = u32::from_be_bytes(png[20..24].try_into().expect("png height bytes"));
    Ok((width, height))
}
