use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── Status Enums ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    #[default]
    Draft,
    Active,
    Complete,
    Blocked,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    #[default]
    Pending,
    Active,
    Verified,
    Failed,
    Skipped,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TestKind {
    #[default]
    Unit,
    Integration,
    E2e,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    #[default]
    Planned,
    Written,
    Passing,
    Failing,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Pass,
    Fail,
    Skipped,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEventType {
    ProjectInitialized,
    FeatureCreated,
    FeatureActivated,
    PhaseCreated,
    PhaseActivated,
    TestAdded,
    ImplementationRun,
    VerificationRun,
    PhaseVerified,
    PhaseFailed,
    PhaseAdvanced,
}

// ─── Core Domain Models ───────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeatureSpec {
    pub id: String,
    pub title: String,
    pub purpose: String,
    #[serde(default)]
    pub outcomes: Vec<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub status: FeatureStatus,
    pub current_phase: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhaseSpec {
    pub id: String,
    pub feature_id: String,
    pub title: String,
    pub goal: String,
    pub order: u32,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub forbidden_paths: Vec<String>,
    #[serde(default)]
    pub required_tests: Vec<String>,
    #[serde(default)]
    pub status: PhaseStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestSpec {
    pub id: String,
    pub feature_id: String,
    pub phase_id: String,
    pub path: String,
    #[serde(default)]
    pub purpose_refs: Vec<String>,
    #[serde(default)]
    pub kind: TestKind,
    #[serde(default)]
    pub status: TestStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TestManifest {
    #[serde(default)]
    pub tests: Vec<TestSpec>,
}

// ─── Agent Models ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTask {
    pub feature_id: String,
    pub phase_id: String,
    pub agent: String,
    pub prompt: String,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentRunResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

// ─── Verification ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationResult {
    pub status: VerificationStatus,
    pub test_command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

// ─── Ledger ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LedgerEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: LedgerEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl LedgerEvent {
    pub fn new(event_type: LedgerEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            feature_id: None,
            phase_id: None,
            agent: None,
            success: None,
            message: None,
        }
    }

    pub fn with_feature(mut self, feature_id: impl Into<String>) -> Self {
        self.feature_id = Some(feature_id.into());
        self
    }

    pub fn with_phase(mut self, phase_id: impl Into<String>) -> Self {
        self.phase_id = Some(phase_id.into());
        self
    }

    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

// ─── Project State ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProjectState {
    pub active_feature: Option<String>,
    pub active_phase: Option<String>,
}
