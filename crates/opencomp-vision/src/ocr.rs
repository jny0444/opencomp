use std::io::Write;

use opencomp_core::error::OpenCompCoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn read(png: &[u8]) -> Result<Vec<TextBlock>, OpenCompCoreError> {
    let mut file = tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    file.write_all(png)
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    file.flush()
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;

    let mut tess = leptess::LepTess::new(None, "eng")
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
    tess.set_image(file.path())
        .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;

    let Some(boxes) = tess.get_component_boxes(leptess::capi::TessPageIteratorLevel_RIL_WORD, true)
    else {
        return Ok(Vec::new());
    };

    let mut blocks = Vec::new();
    for word in &boxes {
        tess.set_rectangle_from_box(&word);
        let text = tess
            .get_utf8_text()
            .map_err(|error| OpenCompCoreError::Computer(error.to_string()))?;
        let text = text.trim().to_owned();
        if text.is_empty() {
            continue;
        }
        let geometry = word.get_geometry();
        let Ok(x) = u32::try_from(geometry.x) else {
            continue;
        };
        let Ok(y) = u32::try_from(geometry.y) else {
            continue;
        };
        let Ok(width) = u32::try_from(geometry.w) else {
            continue;
        };
        let Ok(height) = u32::try_from(geometry.h) else {
            continue;
        };
        if width == 0 || height == 0 {
            continue;
        }
        blocks.push(TextBlock {
            text,
            x,
            y,
            width,
            height,
        });
    }
    Ok(blocks)
}
