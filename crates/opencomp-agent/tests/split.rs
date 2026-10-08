use std::sync::{Arc, Mutex};

use opencomp_agent::Agent;
use opencomp_core::action::Action;
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_core::observation::Observation;
use opencomp_llm::{Completer, Reply, SplitModel};

fn hash(bytes: &[u8]) -> u64 {
    u64::from(bytes.first().copied().unwrap_or(0))
}

struct Screen {
    png: Vec<u8>,
}

impl Computer for Screen {
    async fn screenshot(&mut self) -> Result<Observation, OpenCompCoreError> {
        Ok(Observation {
            png: self.png.clone(),
            width: 10,
            height: 10,
            screen_width: 10,
            screen_height: 10,
        })
    }

    async fn act(&mut self, action: &Action) -> Result<(), OpenCompCoreError> {
        if matches!(action, Action::Done { .. }) {
            return Err(OpenCompCoreError::InvalidAction(
                "done is not a computer action".to_owned(),
            ));
        }
        self.png = vec![9];
        Ok(())
    }
}

struct Queue {
    label: &'static str,
    replies: Vec<String>,
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl Completer for Queue {
    async fn complete(
        &mut self,
        _text: &str,
        _png: Option<&[u8]>,
    ) -> Result<Reply, OpenCompCoreError> {
        self.calls.lock().unwrap().push(self.label);
        let text = self.replies.remove(0);
        Ok(Reply {
            text,
            output_bytes: 4,
            output_tokens: Some(1),
        })
    }
}

#[tokio::test]
async fn a_click_then_done_uses_one_grounder_call_per_click() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let model = SplitModel::new(
        Queue {
            label: "planner",
            replies: vec![
                r#"{"subgoal":"center","actions":[]}"#.to_owned(),
                r#"{"subgoal":"","actions":[{"Done":{"result":"finished"}}]}"#.to_owned(),
            ],
            calls: Arc::clone(&calls),
        },
        Queue {
            label: "grounder",
            replies: vec![
                r#"{"actions":[{"Click":{"point":{"x":1,"y":1},"button":"Left"}}]}"#.to_owned(),
            ],
            calls: Arc::clone(&calls),
        },
        hash,
    );
    let mut agent = Agent::new(Screen { png: vec![1] }, model, 4).with_frame_hash(hash);

    let result = agent.run("click the center, then stop").await.unwrap();

    assert_eq!(result, "finished");
    assert_eq!(
        calls.lock().unwrap().as_slice(),
        ["planner", "grounder", "planner"]
    );
}
