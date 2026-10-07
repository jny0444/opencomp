use opencomp_core::{action::Action, error::OpenCompCoreError};

pub fn instruction(task: &str, image_width: u32, image_height: u32) -> String {
    format!(
        "Task: {task}\n\
         The screenshot is {image_width} by {image_height} pixels.\n\
         Coordinates in the action are in that image, not the raw screen.\n\
         Reply with one JSON object and no markdown. Use exactly one of these shapes. Do not invent names.\n\
         {{\"Click\":{{\"point\":{{\"x\":10,\"y\":20}},\"button\":\"Left\"}}}}\n\
         {{\"DoubleClick\":{{\"x\":10,\"y\":20}}}}\n\
         {{\"Move\":{{\"x\":10,\"y\":20}}}}\n\
         {{\"Drag\":{{\"from\":{{\"x\":1,\"y\":2}},\"to\":{{\"x\":3,\"y\":4}}}}}}\n\
         {{\"Release\":{{\"point\":{{\"x\":12,\"y\":24}},\"button\":\"Left\"}}}}\n\
         {{\"Type\":\"text\"}}\n\
         {{\"Key\":{{\"keys\":[\"Enter\"]}}}}\n\
         {{\"Scroll\":{{\"dx\":0,\"dy\":-3}}}}\n\
         {{\"Wait\":{{\"millis\":200}}}}\n\
         {{\"Done\":{{\"result\":\"finished\"}}}}\n\
         Button is Left, Right, or Middle. A task that moves and then stops is a Move on this reply. Done is a later reply."
    )
}

pub fn user_text(
    task: &str,
    image_width: u32,
    image_height: u32,
    history: &[Action],
) -> Result<String, OpenCompCoreError> {
    let mut text = instruction(task, image_width, image_height);
    if history.is_empty() {
        return Ok(text);
    }
    text.push_str("\n\nActions already performed:\n");
    for action in history {
        let line = serde_json::to_string(action)
            .map_err(|error| OpenCompCoreError::Model(error.to_string()))?;
        text.push_str(&line);
        text.push('\n');
    }
    Ok(text)
}

pub fn parse_action(text: &str) -> Result<Action, OpenCompCoreError> {
    let text = strip_fence(text.trim());
    serde_json::from_str(text).map_err(|error| OpenCompCoreError::Model(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::instruction;

    #[test]
    fn instruction_lists_move_and_done_as_separate_replies() {
        let text = instruction("move the pointer slightly and stop", 100, 50);
        assert!(text.contains("{\"Move\":{\"x\":10,\"y\":20}}"));
        assert!(text.contains("{\"Done\":{\"result\":\"finished\"}}"));
        assert!(text.contains("Do not invent names"));
        assert!(text.contains("Done is a later reply"));
    }
}

fn strip_fence(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_alphanumeric());
    let rest = rest.trim();
    rest.strip_suffix("```").unwrap_or(rest).trim()
}
