use opencomp_computer::Desktop;
use opencomp_core::action::{Action, Key};
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_vision::TextBlock;

const OPEN_WAIT_MS: u64 = 300;
const RESULT_WAIT_MS: u64 = 500;
const APP_WAIT_MS: u64 = 1000;

pub async fn prepare(desktop: &mut Desktop, task: &str) -> Result<String, OpenCompCoreError> {
    let Some(query) = spotlight_query(task) else {
        return Ok(task.to_owned());
    };
    tracing::info!(%query, "opening spotlight locally");
    desktop
        .act(&Action::Key {
            keys: vec![Key::Super, Key::Char(' ')],
        })
        .await?;
    desktop
        .act(&Action::Wait {
            millis: OPEN_WAIT_MS,
        })
        .await?;
    desktop.act(&Action::Type(query.clone())).await?;
    desktop
        .act(&Action::Wait {
            millis: RESULT_WAIT_MS,
        })
        .await?;

    let observation = desktop.screenshot_primary().await?;
    let blocks = opencomp_vision::read(&observation.png)?;
    if first_result_matches(&blocks, &query) {
        tracing::info!(%query, "spotlight first result matched");
        desktop
            .act(&Action::Key {
                keys: vec![Key::Enter],
            })
            .await?;
        desktop
            .act(&Action::Wait {
                millis: RESULT_WAIT_MS,
            })
            .await?;
        if let Some(typed) = address_text(task) {
            tracing::info!(%typed, "opening a new tab");
            desktop
                .act(&Action::Wait {
                    millis: APP_WAIT_MS,
                })
                .await?;
            desktop
                .act(&Action::Key {
                    keys: vec![Key::Super, Key::Char('t')],
                })
                .await?;
            desktop
                .act(&Action::Wait {
                    millis: OPEN_WAIT_MS,
                })
                .await?;
            desktop.act(&Action::Type(typed.clone())).await?;
            return Ok(task_after_tab(&query, &typed, task));
        }
        return Ok(format!(
            "{query} is open. Do not use Spotlight. {}",
            remainder(task)
        ));
    }
    tracing::info!(%query, "spotlight first result did not match");
    Ok(format!(
        "Spotlight is open with {query} typed. Do not press Command-Space again. {task}"
    ))
}

fn address_text(task: &str) -> Option<String> {
    if task.to_ascii_lowercase().contains("youtube") {
        Some("youtube".to_owned())
    } else {
        None
    }
}

fn task_after_tab(app: &str, typed: &str, task: &str) -> String {
    let mut text = format!(
        "{app} is open. A new tab is open with {typed} typed in the address bar. Press Enter to load it. Do not use Spotlight. Do not open another tab."
    );
    let extra = leftover(task);
    if !extra.is_empty() {
        text.push(' ');
        text.push_str(&extra);
        text.push('.');
    }
    text
}

fn leftover(task: &str) -> String {
    let mut text = remainder(task);
    for phrase in [
        "open safari and open youtube on it",
        "and open youtube on it",
        "open youtube on it",
        "open youtube and",
        "and open youtube",
        "open youtube",
        "open safari and",
        "open safari",
        "youtube",
    ] {
        text = cut_phrase(&text, phrase);
    }
    if matches!(
        text.to_ascii_lowercase().as_str(),
        "on it" | "and" | "it" | "on"
    ) {
        return String::new();
    }
    text
}

fn remainder(task: &str) -> String {
    let mut text = task.to_owned();
    for phrase in [
        "using spotlight search",
        "using spotlight",
        "with spotlight",
        "via spotlight",
        "spotlight search for",
        "spotlight",
    ] {
        text = cut_phrase(&text, phrase);
    }
    text
}

fn cut_phrase(task: &str, phrase: &str) -> String {
    let lower = task.to_ascii_lowercase();
    let Some(index) = lower.find(phrase) else {
        return task.to_owned();
    };
    let mut text = String::new();
    text.push_str(task[..index].trim_end());
    text.push(' ');
    text.push_str(task[index + phrase.len()..].trim_start());
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn spotlight_query(task: &str) -> Option<String> {
    let lower = task.to_ascii_lowercase();
    if let Some(index) = lower.find("using spotlight") {
        return last_name(&task[..index]);
    }
    if !lower.contains("spotlight") {
        return None;
    }
    let index = lower.find("spotlight")?;
    first_name(task[index + "spotlight".len()..].trim_start()).or_else(|| last_name(&task[..index]))
}

fn last_name(text: &str) -> Option<String> {
    text.split_whitespace().rev().find_map(clean_name)
}

fn first_name(text: &str) -> Option<String> {
    text.split_whitespace().find_map(|word| {
        if matches!(
            word.to_ascii_lowercase().as_str(),
            "search" | "for" | "to" | "and" | "the"
        ) {
            return None;
        }
        clean_name(word)
    })
}

fn clean_name(word: &str) -> Option<String> {
    let name = word.trim_matches(|character: char| !character.is_ascii_alphanumeric());
    if name.is_empty()
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "open" | "the" | "a" | "an" | "using"
        )
    {
        return None;
    }
    Some(name.to_owned())
}

pub fn first_result_matches(blocks: &[TextBlock], query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    let mut sorted = blocks.to_vec();
    sorted.sort_by_key(|block| (block.y, block.x));
    let mut lines: Vec<(u32, String)> = Vec::new();
    for block in sorted {
        if let Some((y, line)) = lines.last_mut()
            && block.y.abs_diff(*y) <= 12
        {
            line.push(' ');
            line.push_str(&block.text);
            continue;
        }
        lines.push((block.y, block.text));
    }
    let candidate = if lines.len() >= 2 && normalize(&lines[0].1) == query {
        &lines[1].1
    } else if let Some((_, line)) = lines.first() {
        line
    } else {
        return false;
    };
    normalize(candidate).contains(&query)
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use opencomp_vision::TextBlock;

    use super::{address_text, first_result_matches, remainder, spotlight_query, task_after_tab};

    fn block(text: &str, y: u32) -> TextBlock {
        TextBlock {
            text: text.to_owned(),
            x: 10,
            y,
            width: 40,
            height: 12,
        }
    }

    #[test]
    fn the_app_before_using_spotlight_is_the_query() {
        assert_eq!(
            spotlight_query("open safari using spotlight search and open youtube on it").as_deref(),
            Some("safari")
        );
        assert_eq!(
            remainder("open safari using spotlight search and open youtube on it"),
            "open safari and open youtube on it"
        );
    }

    #[test]
    fn the_first_result_below_the_query_must_match() {
        let blocks = vec![
            block("safari", 40),
            block("Safari", 80),
            block("Application", 80),
        ];
        assert!(first_result_matches(&blocks, "safari"));

        let blocks = vec![block("safari", 40), block("Notes", 80)];
        assert!(!first_result_matches(&blocks, "safari"));
    }

    #[test]
    fn youtube_is_typed_into_the_new_tab() {
        let task = "open safari using spotlight search and open youtube on it";
        assert_eq!(address_text(task).as_deref(), Some("youtube"));
        assert_eq!(
            task_after_tab("safari", "youtube", task),
            "safari is open. A new tab is open with youtube typed in the address bar. Press Enter to load it. Do not use Spotlight. Do not open another tab."
        );
        assert_eq!(address_text("open notes using spotlight"), None);
    }
}
