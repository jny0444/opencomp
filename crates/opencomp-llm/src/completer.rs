use opencomp_core::error::OpenCompCoreError;

pub struct Reply {
    pub text: String,
    pub output_bytes: usize,
    pub output_tokens: Option<u32>,
}

pub trait Completer {
    async fn complete(
        &mut self,
        text: &str,
        png: Option<&[u8]>,
    ) -> Result<Reply, OpenCompCoreError>;
}
