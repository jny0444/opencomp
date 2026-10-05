use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenCompCoreError {
    #[error("Screenshot and Input failure: `{0}`")]
    Computer(String),

    #[error("Provider and Parse failure: `{0}`")]
    Model(String),

    #[error("Invalid action: `{0}`")]
    InvalidAction(String),
}