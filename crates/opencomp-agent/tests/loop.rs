mod common;

use opencomp_agent::Agent;
use opencomp_core::action::{Action, MouseButton, Point};
use opencomp_llm::ScriptedModel;

use common::FakeComputer;

fn click() -> Action {
    Action::Click {
        point: Point { x: 1, y: 1 },
        button: MouseButton::Left,
    }
}

#[tokio::test]
async fn run_returns_done_and_records_the_click() {
    let (computer, seen) = FakeComputer::new(2, 2);
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
    let (computer, seen) = FakeComputer::new(2, 2);
    let model = ScriptedModel::new(vec![click()]);
    let mut agent = Agent::new(computer, model, 1);

    let error = agent.run("task").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Provider and Parse failure: `step limit of 1 was hit`"
    );
    assert_eq!(seen.lock().unwrap().as_slice(), &[click()]);
}
