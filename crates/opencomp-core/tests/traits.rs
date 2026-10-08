use opencomp_core::action::{Action, MouseButton, Point};
use opencomp_core::computer::Computer;
use opencomp_core::error::OpenCompCoreError;
use opencomp_core::model::{Model, Turn};
use opencomp_core::observation::Observation;

struct FakeComputer {
    seen: Vec<Action>,
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
        if matches!(action, Action::Done { .. }) {
            return Err(OpenCompCoreError::InvalidAction(
                "done is not a computer action".to_owned(),
            ));
        }
        self.seen.push(action.clone());
        Ok(())
    }
}

struct ScriptedModel {
    actions: Vec<Action>,
}

impl Model for ScriptedModel {
    async fn next_action(
        &mut self,
        _task: &str,
        _observation: &Observation,
        _history: &[Action],
    ) -> Result<Turn, OpenCompCoreError> {
        let action = self
            .actions
            .pop()
            .ok_or_else(|| OpenCompCoreError::Model("no scripted action left".to_owned()))?;
        Ok(Turn {
            action,
            follow: Vec::new(),
            reasoning: None,
            output_bytes: 0,
            output_tokens: None,
        })
    }
}

#[tokio::test]
async fn computer_records_a_click_and_refuses_done() {
    let mut computer = FakeComputer { seen: Vec::new() };
    let observation = computer.screenshot().await.unwrap();
    assert_eq!(observation.width, 1);
    assert_eq!(observation.height, 1);
    assert_eq!(observation.png, vec![0]);

    let click = Action::Click {
        point: Point { x: 1, y: 1 },
        button: MouseButton::Left,
    };
    computer.act(&click).await.unwrap();
    assert_eq!(computer.seen, vec![click]);

    let error = computer
        .act(&Action::Done {
            result: "finished".to_owned(),
        })
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Invalid action: `done is not a computer action`"
    );
    assert_eq!(computer.seen.len(), 1);
}

#[tokio::test]
async fn model_returns_scripted_actions_then_errors() {
    let mut model = ScriptedModel {
        actions: vec![
            Action::Done {
                result: "finished".to_owned(),
            },
            Action::Type("hi".to_owned()),
        ],
    };
    let observation = Observation {
        png: Vec::new(),
        width: 0,
        height: 0,
        screen_width: 0,
        screen_height: 0,
    };

    let first = model
        .next_action("type hi", &observation, &[])
        .await
        .unwrap();
    assert_eq!(first.action, Action::Type("hi".to_owned()));
    assert_eq!(first.reasoning, None);

    let second = model
        .next_action("type hi", &observation, &[first.action])
        .await
        .unwrap();
    assert_eq!(
        second.action,
        Action::Done {
            result: "finished".to_owned(),
        }
    );

    let error = model
        .next_action("type hi", &observation, &[])
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Provider and Parse failure: `no scripted action left`"
    );
}
