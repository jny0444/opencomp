use std::sync::{Arc, Mutex};

use opencomp_agent::Agent;
use opencomp_core::action::{Action, MouseButton, Point};
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_core::observation::Observation;
use opencomp_llm::ScriptedModel;

struct FakeComputer {
    seen: Arc<Mutex<Vec<Action>>>,
}

impl FakeComputer {
    fn new() -> (Self, Arc<Mutex<Vec<Action>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                seen: Arc::clone(&seen),
            },
            seen,
        )
    }
}

impl Computer for FakeComputer {
    async fn screenshot(&mut self) -> Result<Observation, OpenCompCoreError> {
        Ok(Observation {
            png: vec![0],
            width: 1,
            height: 1,
            screen_width: 1,
            screen_height: 1,
        })
    }

    async fn act(&mut self, action: &Action) -> Result<(), OpenCompCoreError> {
        self.seen.lock().unwrap().push(action.clone());
        Ok(())
    }
}

fn click() -> Action {
    Action::Click {
        point: Point { x: 1, y: 1 },
        button: MouseButton::Left,
    }
}

#[tokio::test]
async fn run_returns_done_and_records_the_click() {
    let (computer, seen) = FakeComputer::new();
    let model = ScriptedModel::new(vec![
        click(),
        Action::Done {
            result: "finished".to_owned(),
        },
    ]);
    let mut agent = Agent::new(computer, model, 2);

    let result = agent.run("task").await.unwrap();

    assert_eq!(result, "finished");
    assert_eq!(seen.lock().unwrap().as_slice(), &[click()]);
}

#[tokio::test]
async fn run_stops_when_the_step_limit_is_hit() {
    let (computer, seen) = FakeComputer::new();
    let model = ScriptedModel::new(vec![click()]);
    let mut agent = Agent::new(computer, model, 1);

    let error = agent.run("task").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Provider and Parse failure: `step limit of 1 was hit`"
    );
    assert_eq!(seen.lock().unwrap().as_slice(), &[click()]);
}
