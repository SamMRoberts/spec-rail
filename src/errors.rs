use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum SpeRailError {
    #[error("project not initialized — run `specrail init` first")]
    NotInitialized,

    #[error("feature not found: {0}")]
    FeatureNotFound(String),

    #[error("outcome not found: {outcome_id} for feature {feature_id}")]
    OutcomeNotFound {
        feature_id: String,
        outcome_id: String,
    },

    #[error("no active feature set — use `specrail feature activate <id>`")]
    NoActiveFeature,

    #[error("no active outcome set — use `specrail outcome activate <feature-id> <outcome-id>`")]
    NoActiveOutcome,

    #[error("outcome gate rejected: {0}")]
    OutcomeGateRejected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
