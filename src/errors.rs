use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum SpeRailError {
    #[error("project not initialized — run `specrail init` first")]
    NotInitialized,

    #[error("feature not found: {0}")]
    FeatureNotFound(String),

    #[error("phase not found: {phase_id} for feature {feature_id}")]
    PhaseNotFound {
        feature_id: String,
        phase_id: String,
    },

    #[error("no active feature set — use `specrail feature activate <id>`")]
    NoActiveFeature,

    #[error("no active phase set — use `specrail phase activate <feature-id> <phase-id>`")]
    NoActivePhase,

    #[error("phase gate rejected: {0}")]
    PhaseGateRejected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
