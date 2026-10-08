use opencomp_core::action::Action;
use opencomp_core::model::Model;
use opencomp_core::observation::Observation;
use opencomp_llm::AgentRouterModel;
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

    let mut model = AgentRouterModel::new("test-key".to_owned(), "gpt-4o".to_owned())
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
    let requests = server.received_requests().await.unwrap();
    let user_agent = requests[0]
        .headers
        .get("user-agent")
        .map(|value| value.to_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(user_agent, "claude-cli/1.0.0 (external, cli)");
}
