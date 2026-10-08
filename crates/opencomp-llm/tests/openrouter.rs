use opencomp_core::model::Model;
use opencomp_core::observation::Observation;
use opencomp_llm::OpenRouterModel;
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
async fn rejects_a_body_that_is_not_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "not json"
                }
            }]
        })))
        .mount(&server)
        .await;

    let mut model = OpenRouterModel::new("test-key".to_owned(), "qwen/qwen3.8-27b".to_owned())
        .with_base_url(server.uri());
    let error = model
        .next_action("task", &observation(), &[])
        .await
        .unwrap_err();

    assert!(
        error.to_string().starts_with("Provider and Parse failure:"),
        "{error}"
    );
}

#[tokio::test]
async fn includes_the_provider_message_on_http_404() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "message": "No endpoints found that support image input",
                "code": 404
            }
        })))
        .mount(&server)
        .await;

    let mut model = OpenRouterModel::new("test-key".to_owned(), "text-only".to_owned())
        .with_base_url(server.uri());
    let error = model
        .next_action("task", &observation(), &[])
        .await
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("No endpoints found that support image input"),
        "{error}"
    );
}
