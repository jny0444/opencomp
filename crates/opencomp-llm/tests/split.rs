use std::sync::{Arc, Mutex};

use opencomp_core::action::{Action, Key, MouseButton, Point};
use opencomp_core::error::OpenCompCoreError;
use opencomp_core::model::Model;
use opencomp_core::observation::Observation;
use opencomp_llm::{Completer, Reply, SplitModel};

fn hash(bytes: &[u8]) -> u64 {
    u64::from(bytes.first().copied().unwrap_or(0))
}

fn observation(pixel: u8) -> Observation {
    Observation {
        png: vec![pixel],
        width: 10,
        height: 10,
        screen_width: 10,
        screen_height: 10,
    }
}

fn click() -> String {
    r#"{"actions":[{"Click":{"point":{"x":1,"y":1},"button":"Left"}}]}"#.to_owned()
}

struct Queue {
    label: &'static str,
    replies: Vec<String>,
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl Completer for Queue {
    async fn complete(
        &mut self,
        text: &str,
        png: Option<&[u8]>,
    ) -> Result<Reply, OpenCompCoreError> {
        assert!(png.is_some(), "{}", self.label);
        if self.label == "planner" {
            assert!(text.contains("Task:"), "{text}");
        } else {
            assert!(text.contains("Target: the round target"), "{text}");
            assert!(!text.contains("hit the mark"), "{text}");
        }
        self.calls.lock().unwrap().push(self.label);
        let text = self.replies.remove(0);
        Ok(Reply {
            text,
            output_bytes: 4,
            output_tokens: Some(1),
        })
    }
}

fn split(
    planner: Vec<String>,
    grounder: Vec<String>,
) -> (SplitModel<Queue, Queue>, Arc<Mutex<Vec<&'static str>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let model = SplitModel::new(
        Queue {
            label: "planner",
            replies: planner,
            calls: Arc::clone(&calls),
        },
        Queue {
            label: "grounder",
            replies: grounder,
            calls: Arc::clone(&calls),
        },
        hash,
    );
    (model, calls)
}

#[tokio::test]
async fn a_click_calls_the_planner_then_the_grounder() {
    let (mut model, calls) = split(
        vec![r#"{"subgoal":"the round target","actions":[]}"#.to_owned()],
        vec![click()],
    );

    let turn = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();

    assert_eq!(
        turn.action,
        Action::Click {
            point: Point { x: 1, y: 1 },
            button: MouseButton::Left,
        }
    );
    assert_eq!(calls.lock().unwrap().as_slice(), ["planner", "grounder"]);
}

#[tokio::test]
async fn a_landed_click_plans_the_next_step_without_grounding() {
    let (mut model, calls) = split(
        vec![
            r#"{"subgoal":"the round target","actions":[]}"#.to_owned(),
            r#"{"subgoal":"","actions":[{"Done":{"result":"finished"}}]}"#.to_owned(),
        ],
        vec![click()],
    );

    let click = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();
    let done = model
        .next_action("hit the mark", &observation(2), &[click.action])
        .await
        .unwrap();

    assert_eq!(
        done.action,
        Action::Done {
            result: "finished".to_owned(),
        }
    );
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["planner", "grounder", "planner"]
    );
}

#[tokio::test]
async fn an_unchanged_frame_goes_back_to_the_planner() {
    let (mut model, calls) = split(
        vec![
            r#"{"subgoal":"the round target","actions":[]}"#.to_owned(),
            r#"{"subgoal":"the round target","actions":[{"Done":{"result":"missed"}}]}"#.to_owned(),
        ],
        vec![click()],
    );

    let click = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();
    let done = model
        .next_action("hit the mark", &observation(1), &[click.action])
        .await
        .unwrap();

    assert!(matches!(done.action, Action::Done { .. }));
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["planner", "grounder", "planner"]
    );
}

#[tokio::test]
async fn a_decline_asks_the_planner_again() {
    let (mut model, calls) = split(
        vec![
            r#"{"subgoal":"the round target","actions":[]}"#.to_owned(),
            r#"{"subgoal":"","actions":[{"Done":{"result":"nowhere"}}]}"#.to_owned(),
        ],
        vec![r#"{"decline":true}"#.to_owned()],
    );

    let turn = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();

    assert_eq!(
        turn.action,
        Action::Done {
            result: "nowhere".to_owned(),
        }
    );
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["planner", "grounder", "planner"]
    );
}

#[tokio::test]
async fn a_text_burst_skips_the_grounder() {
    let (mut model, calls) = split(
        vec![
            r#"{"subgoal":"the round target","actions":[{"Type":"hi"},{"Done":{"result":"typed"}}]}"#.to_owned(),
        ],
        vec![],
    );

    let typed = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();
    let done = model
        .next_action("hit the mark", &observation(2), &[typed.action.clone()])
        .await
        .unwrap();

    assert_eq!(typed.action, Action::Type("hi".to_owned()));
    assert_eq!(
        done.action,
        Action::Done {
            result: "typed".to_owned(),
        }
    );
    assert_eq!(calls.lock().unwrap().as_slice(), ["planner"]);
}

#[tokio::test]
async fn an_empty_subgoal_is_planned_again() {
    let (mut model, calls) = split(
        vec![
            r#"{"subgoal":"","actions":[{"Type":"text"}]}"#.to_owned(),
            r#"{"subgoal":"the round target","actions":[]}"#.to_owned(),
        ],
        vec![click()],
    );

    let turn = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();

    assert_eq!(
        turn.action,
        Action::Click {
            point: Point { x: 1, y: 1 },
            button: MouseButton::Left,
        }
    );
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["planner", "planner", "grounder"]
    );
}

#[tokio::test]
async fn an_empty_subgoal_still_runs_a_key_chord() {
    let (mut model, calls) = split(
        vec![r#"{"subgoal":"","actions":[{"Key":{"keys":["Super",{"Char":" "}]}}]}"#.to_owned()],
        vec![],
    );

    let turn = model
        .next_action("hit the mark", &observation(1), &[])
        .await
        .unwrap();

    assert_eq!(
        turn.action,
        Action::Key {
            keys: vec![Key::Super, Key::Char(' ')],
        }
    );
    assert_eq!(calls.lock().unwrap().as_slice(), ["planner"]);
}
