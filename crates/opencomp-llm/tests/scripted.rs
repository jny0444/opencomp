use opencomp_core::action::{Action, Point};
use opencomp_core::model::Model;
use opencomp_core::observation::Observation;
use opencomp_llm::ScriptedModel;

#[tokio::test]
async fn returns_move_type_done_then_errors() {
    let mut model = ScriptedModel::new(vec![
        Action::Move(Point { x: 1, y: 1 }),
        Action::Type("hi".to_owned()),
        Action::Done {
            result: "finished".to_owned(),
        },
    ]);
    let observation = Observation {
        png: Vec::new(),
        width: 0,
        height: 0,
        screen_width: 0,
        screen_height: 0,
    };

    let first = model.next_action("task", &observation, &[]).await.unwrap();
    assert_eq!(first.action, Action::Move(Point { x: 1, y: 1 }));
    assert_eq!(first.reasoning, None);

    let second = model
        .next_action("task", &observation, &[first.action.clone()])
        .await
        .unwrap();
    assert_eq!(second.action, Action::Type("hi".to_owned()));
    assert_eq!(second.reasoning, None);

    let third = model.next_action("task", &observation, &[]).await.unwrap();
    assert_eq!(
        third.action,
        Action::Done {
            result: "finished".to_owned(),
        }
    );

    let error = model
        .next_action("task", &observation, &[])
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Provider and Parse failure: `no scripted action left`"
    );
}
