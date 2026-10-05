use opencomp_core::{action::Action, computer::Computer, error::OpenCompCoreError};
use xcap::Monitor;

use crate::input;

pub struct Desktop {
    enigo: enigo::Enigo,
    scale_factor: f32,
}

impl Desktop {
    pub fn new() -> Result<Self, OpenCompCoreError> {
        let enigo = enigo::Enigo::new(&enigo::Settings::default())
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        Ok(Self {
            enigo: enigo,
            scale_factor: 0.0,
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
        let monitors =
            xcap::Monitor::all().map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        let monitor = monitors
            .into_iter()
            .find(|m| m.is_primary().unwrap_or(false))
            .ok_or_else(|| OpenCompCoreError::Computer("no primary monitor".to_string()))?;

        let image = monitor
            .capture_image()
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        self.scale_factor = monitor
            .scale_factor()
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        let width = image.width();
        let height = image.height();
        let mut png = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| OpenCompCoreError::Computer(e.to_string()))?;

        Ok(opencomp_core::observation::Observation {
            png: png.into_inner(),
            width,
            height,
        })
    }
}
