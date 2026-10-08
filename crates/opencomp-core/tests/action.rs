use opencomp_core::action::{Action, Key, MouseButton, Point};

fn assert_round_trip(action: &Action, json: &str) {
    let encoded = serde_json::to_string(action).unwrap();
    assert_eq!(encoded, json);

    let decoded: Action = serde_json::from_str(json).unwrap();
    assert_eq!(&decoded, action);
}

#[test]
fn click_round_trips_through_json() {
    assert_round_trip(
        &Action::Click {
            point: Point { x: 10, y: 20 },
            button: MouseButton::Left,
        },
        r#"{"Click":{"point":{"x":10,"y":20},"button":"Left"}}"#,
    );
}

#[test]
fn mouse_buttons_round_trip_through_json() {
    assert_round_trip(
        &Action::Click {
            point: Point { x: 1, y: 2 },
            button: MouseButton::Right,
        },
        r#"{"Click":{"point":{"x":1,"y":2},"button":"Right"}}"#,
    );
    assert_round_trip(
        &Action::Click {
            point: Point { x: 1, y: 2 },
            button: MouseButton::Middle,
        },
        r#"{"Click":{"point":{"x":1,"y":2},"button":"Middle"}}"#,
    );
}

#[test]
fn double_click_round_trips_through_json() {
    assert_round_trip(
        &Action::DoubleClick(Point { x: 4, y: 5 }),
        r#"{"DoubleClick":{"x":4,"y":5}}"#,
    );
}

#[test]
fn move_round_trips_through_json() {
    assert_round_trip(
        &Action::Move(Point { x: 8, y: 9 }),
        r#"{"Move":{"x":8,"y":9}}"#,
    );
}

#[test]
fn drag_round_trips_through_json() {
    assert_round_trip(
        &Action::Drag {
            from: Point { x: 1, y: 1 },
            to: Point { x: 30, y: 40 },
        },
        r#"{"Drag":{"from":{"x":1,"y":1},"to":{"x":30,"y":40}}}"#,
    );
}

#[test]
fn release_round_trips_through_json() {
    assert_round_trip(
        &Action::Release {
            point: Point { x: 12, y: 24 },
            button: MouseButton::Left,
        },
        r#"{"Release":{"point":{"x":12,"y":24},"button":"Left"}}"#,
    );
}

#[test]
fn type_round_trips_through_json() {
    assert_round_trip(&Action::Type("hello".to_owned()), r#"{"Type":"hello"}"#);
}

#[test]
fn key_chord_round_trips_through_json() {
    assert_round_trip(
        &Action::Key {
            keys: vec![Key::Super, Key::Char('c')],
        },
        r#"{"Key":{"keys":["Super",{"Char":"c"}]}}"#,
    );
}

#[test]
fn scroll_keeps_negative_offsets() {
    assert_round_trip(
        &Action::Scroll { dx: -4, dy: -8 },
        r#"{"Scroll":{"dx":-4,"dy":-8}}"#,
    );
}

#[test]
fn wait_stores_milliseconds() {
    assert_round_trip(&Action::Wait { millis: 500 }, r#"{"Wait":{"millis":500}}"#);
}

#[test]
fn done_round_trips_through_json() {
    assert_round_trip(
        &Action::Done {
            result: "opened Notes".to_owned(),
        },
        r#"{"Done":{"result":"opened Notes"}}"#,
    );
}

#[test]
fn unknown_action_is_rejected() {
    let error = serde_json::from_str::<Action>(r#"{"Hover":{"x":1,"y":2}}"#).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("Hover"), "{message}");
}
