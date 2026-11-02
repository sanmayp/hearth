//! One `Llm` trait for all model access.
//! The provider is config, not scattered code.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// One chat request to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatReq {
    /// Stable system prefix (cached by the provider).
    pub system: String,
    /// Short summary, never raw history.
    pub summary: String,
    /// The user message.
    pub user: String,
    /// Max tokens for this call. Shown in the dev panel.
    pub budget: u32,
}

/// One chat answer from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResp {
    /// The answer text.
    pub text: String,
    /// Tokens used (when the provider reports them).
    pub tokens: Option<u32>,
}

/// The single model interface.
#[async_trait]
pub trait Llm: Send + Sync {
    /// Complete one chat request.
    async fn complete(&self, req: ChatReq) -> Result<ChatResp>;
    /// Provider name for logs and the dev panel.
    fn name(&self) -> &'static str;
}

/// Rules-only fallback. Used when AI is off for a module.
/// It never calls a model. It returns a fixed plain reply.
pub struct NoopLlm;

#[async_trait]
impl Llm for NoopLlm {
    async fn complete(&self, _req: ChatReq) -> Result<ChatResp> {
        Ok(ChatResp {
            text: "AI is off for this module. It works with rules only.".into(),
            tokens: Some(0),
        })
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

/// Muse (Meta API) behind the `Llm` trait.
/// Reads endpoint and key from env at build time.
pub struct MetaMuse {
    /// API endpoint.
    pub endpoint: String,
    /// Key presence only; never logged.
    pub has_key: bool,
}

impl MetaMuse {
    /// Build from env: `META_ENDPOINT`, `META_API_KEY`.
    pub fn from_env() -> Result<Self> {
        let endpoint = std::env::var("META_ENDPOINT")
            .map_err(|_| Error::Config("META_ENDPOINT not set".into()))?;
        let key = std::env::var("META_API_KEY").unwrap_or_default();
        if key.is_empty() {
            return Err(Error::Config("META_API_KEY not set".into()));
        }
        Ok(Self {
            endpoint,
            has_key: true,
        })
    }
}

#[async_trait]
impl Llm for MetaMuse {
    async fn complete(&self, _req: ChatReq) -> Result<ChatResp> {
        // Transport lands with the first AI-enabled module.
        // Until then this type checks config only.
        Err(Error::Model("muse transport not wired yet".into()))
    }

    fn name(&self) -> &'static str {
        "meta-muse"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_uses_zero_tokens() {
        let llm = NoopLlm;
        let resp = llm
            .complete(ChatReq {
                system: "s".into(),
                summary: "u".into(),
                user: "hi".into(),
                budget: 100,
            })
            .await
            .unwrap();
        assert_eq!(resp.tokens, Some(0));
        assert_eq!(llm.name(), "noop");
    }
}
