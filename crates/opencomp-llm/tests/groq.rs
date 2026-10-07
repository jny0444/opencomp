use opencomp_core::action::Action;
use opencomp_core::model::Model;
use opencomp_core::observation::Observation;
use opencomp_llm::GroqModel;
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
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "{\"Done\":{\"result\":\"ok\"}}"
                }
            }]
        })))
        .mount(&server)
        .await;

    let mut model = GroqModel::new("test-key".to_owned(), "qwen/qwen3.8-27b".to_owned())
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
