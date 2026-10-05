use opencomp_core::error::OpenCompCoreError;

#[test]
fn computer_error_includes_the_detail() {
    let error = OpenCompCoreError::Computer("capture failed".to_owned());
    assert_eq!(
        error.to_string(),
        "Screenshot and Input failure: `capture failed`"
    );
}

#[test]
fn model_error_includes_the_detail() {
    let error = OpenCompCoreError::Model("empty response".to_owned());
    assert_eq!(
        error.to_string(),
        "Provider and Parse failure: `empty response`"
    );
}

#[test]
fn invalid_action_includes_the_detail() {
    let error = OpenCompCoreError::InvalidAction("done is not a computer action".to_owned());
    assert_eq!(
        error.to_string(),
        "Invalid action: `done is not a computer action`"
    );
}
