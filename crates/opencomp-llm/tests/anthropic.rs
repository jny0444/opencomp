use opencomp_core::action::Action;
use opencomp_core::model::Model;
use opencomp_core::observation::Observation;
use opencomp_llm::AnthropicModel;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn observation() -> Observation {
    Observation {
        png: vec![0],
        width: 1,
        height: 1,
        screen_width: 1,
        screen_height: 1,
    }
}

#[tokio::test]
async fn parses_a_done_action() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{
                "type": "text",
                "text": "{\"Done\":{\"result\":\"ok\"}}"
            }]
        })))
        .mount(&server)
        .await;

    let mut model = AnthropicModel::new("test-key".to_owned(), "claude".to_owned())
        .with_base_url(server.uri());
    let turn = model
        .next_action("task", &observation(), &[])
        .await
        .unwrap();

    assert_eq!(
        turn.action,
        Action::Done {
            result: "ok".to_owned(),
        }
    );
    assert_eq!(turn.reasoning, None);
}

#[tokio::test]
async fn rejects_a_body_that_is_not_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{
                "type": "text",
                "text": "not json"
            }]
        })))
        .mount(&server)
        .await;

    let mut model = AnthropicModel::new("test-key".to_owned(), "claude".to_owned())
        .with_base_url(server.uri());
    let error = model
        .next_action("task", &observation(), &[])
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string().starts_with("Provider and Parse failure:"),
        true,
        "{error}"
    );
}
