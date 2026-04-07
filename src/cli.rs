use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "specrail",
    version,
    about = "Phase-gated, purpose-driven, test-based engineering methodology CLI",
    long_about = None,
)]
pub struct Cli {
    /// Increase verbosity (can be repeated: -v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new specrail project in the current directory
    Init {
        /// Skip the interactive onboarding walkthrough after init
        #[arg(long)]
        no_wizard: bool,
    },

    /// Manage features
    #[command(subcommand)]
    Feature(FeatureCommands),

    /// Manage phases
    #[command(subcommand)]
    Phase(PhaseCommands),

    /// Manage tests in the manifest
    #[command(subcommand)]
    Test(TestCommands),

    /// Run the AI coding agent for the active phase
    Implement {
        /// Override the configured agent (generic-shell | copilot | codex)
        #[arg(long, short)]
        agent: Option<String>,
    },

    /// Run tests to verify the active phase
    Verify,

    /// Advance to the next phase (requires current phase to be verified)
    Advance,

    /// Show project status dashboard
    Status,

    /// Show the audit ledger of events
    Trace {
        /// Show only the last N events
        #[arg(long, short)]
        limit: Option<usize>,
    },
}

// ── Feature sub-commands ──────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum FeatureCommands {
    /// Create a new feature
    New {
        /// Unique feature identifier (e.g. auth-login)
        id: String,
        /// Short human-readable title
        #[arg(long, short)]
        title: String,
        /// Purpose statement: why this feature exists
        #[arg(long, short)]
        purpose: String,
        /// Expected outcomes (repeatable)
        #[arg(long = "outcome", short = 'o')]
        outcomes: Vec<String>,
        /// Constraints (repeatable)
        #[arg(long = "constraint", short = 'c')]
        constraints: Vec<String>,
        /// Non-goals (repeatable)
        #[arg(long = "non-goal", short = 'n')]
        non_goals: Vec<String>,
        /// Feature dependencies (repeatable)
        #[arg(long = "dep", short = 'd')]
        dependencies: Vec<String>,
    },

    /// List all features
    List,

    /// Show details of a feature
    Show {
        /// Feature ID
        id: String,
    },

    /// Mark a feature as the active feature
    Activate {
        /// Feature ID
        id: String,
    },
}

// ── Phase sub-commands ────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum PhaseCommands {
    /// Create a new phase for a feature
    New {
        /// Feature ID
        feature_id: String,
        /// Unique phase identifier (e.g. phase-1-domain)
        phase_id: String,
        /// Short title for the phase
        #[arg(long, short)]
        title: String,
        /// Goal: what this phase accomplishes
        #[arg(long, short)]
        goal: String,
        /// Phase execution order (1-based)
        #[arg(long, short)]
        order: u32,
        /// Phase IDs that must be verified before this one (repeatable)
        #[arg(long = "prereq")]
        prerequisites: Vec<String>,
        /// Glob paths the agent is allowed to modify (repeatable)
        #[arg(long = "allow")]
        allowed_paths: Vec<String>,
        /// Glob paths the agent must NOT touch (repeatable)
        #[arg(long = "forbid")]
        forbidden_paths: Vec<String>,
        /// Test paths required to pass (repeatable)
        #[arg(long = "test")]
        required_tests: Vec<String>,
    },

    /// List phases for a feature
    List {
        /// Feature ID
        feature_id: String,
    },

    /// Show details of a phase
    Show {
        /// Feature ID
        feature_id: String,
        /// Phase ID
        phase_id: String,
    },

    /// Mark a phase as the active phase
    Activate {
        /// Feature ID
        feature_id: String,
        /// Phase ID
        phase_id: String,
    },
}

// ── Test sub-commands ─────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum TestCommands {
    /// Register a test in the manifest
    Add {
        /// Unique test identifier
        id: String,
        /// Feature this test belongs to
        #[arg(long, short = 'f')]
        feature: String,
        /// Phase this test belongs to
        #[arg(long, short = 'P')]
        phase: String,
        /// Relative path to the test file
        #[arg(long, short = 'p')]
        path: String,
        /// Test kind: unit | integration | e2e
        #[arg(long, short, default_value = "unit")]
        kind: String,
        /// Purpose references (repeatable, e.g. outcome:user-can-log-in)
        #[arg(long = "ref", short = 'r')]
        purpose_refs: Vec<String>,
    },

    /// Generate required tests from feature and phase YAML using an AI agent
    Generate {
        /// Override the agent used for generation (defaults to `copilot`)
        #[arg(long, short)]
        agent: Option<String>,
    },

    /// List tests in the manifest
    List {
        /// Filter by feature ID
        #[arg(long, short)]
        feature: Option<String>,
        /// Filter by phase ID
        #[arg(long, short)]
        phase: Option<String>,
    },

    /// Update the status of a test
    SetStatus {
        /// Test ID
        id: String,
        /// New status: planned | written | passing | failing
        status: String,
    },
}
