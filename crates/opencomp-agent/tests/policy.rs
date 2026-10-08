mod common;

use opencomp_agent::Agent;
use opencomp_agent::policy;
use opencomp_core::action::{Action, Key, MouseButton, Point};
use opencomp_llm::ScriptedModel;

use common::FakeComputer;

fn click_at(x: u32, y: u32) -> Action {
    Action::Click {
        point: Point { x, y },
        button: MouseButton::Left,
    }
}

#[tokio::test]
async fn an_out_of_bounds_point_is_refused_before_act() {
    let (computer, seen) = FakeComputer::new(1, 1);
    let model = ScriptedModel::new(vec![
        Action::Move(Point { x: 1, y: 0 }),
        Action::Done {
            result: "should not run".to_owned(),
        },
    ]);
    let mut agent = Agent::new(computer, model, 2);

    let error = agent.run("task").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid action: `point 1,0 is outside 1x1`"
    );
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn super_q_is_refused_before_act() {
    let (computer, seen) = FakeComputer::new(10, 10);
    let model = ScriptedModel::new(vec![Action::Key {
        keys: vec![Key::Super, Key::Char('q')],
    }]);
    let mut agent = Agent::new(computer, model, 1);

    let error = agent.run("task").await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid action: `super+q is not allowed`"
    );
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_click_inside_the_screenshot_is_performed() {
    let (computer, seen) = FakeComputer::new(10, 10);
    let model = ScriptedModel::new(vec![
        click_at(1, 1),
        Action::Done {
            result: "finished".to_owned(),
        },
    ]);
    let mut agent = Agent::new(computer, model, 2);

    let result = agent.run("task").await.unwrap();

    assert_eq!(result, "finished");
    assert_eq!(seen.lock().unwrap().as_slice(), &[click_at(1, 1)]);
}

#[test]
fn super_shortcuts_and_single_keys_are_allowed() {
    let copy = Action::Key {
        keys: vec![Key::Char('c'), Key::Super],
    };
    let quit = Action::Key {
        keys: vec![Key::Char('q')],
    };
    let spotlight = Action::Key {
        keys: vec![Key::Super, Key::Char(' ')],
    };
    let other = Action::Key {
        keys: vec![Key::Super, Key::Char('x')],
    };

    assert!(policy::check(&copy, 10, 10).is_ok());
    assert!(policy::check(&quit, 10, 10).is_ok());
    assert!(policy::check(&spotlight, 10, 10).is_ok());
    assert!(policy::check(&other, 10, 10).is_err());
}
