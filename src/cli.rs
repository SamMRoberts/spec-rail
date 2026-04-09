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

    /// Manage solutions
    #[command(subcommand)]
    Solution(SolutionCommands),

    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Manage components
    #[command(subcommand)]
    Component(ComponentCommands),

    /// Manage outcomes
    #[command(subcommand)]
    Outcome(OutcomeCommands),

    /// Manage tests in the manifest
    #[command(subcommand)]
    Test(TestCommands),

    /// Run the AI coding agent for the active outcome
    Implement {
        /// Override the configured agent (generic-shell | copilot | codex)
        #[arg(long, short)]
        agent: Option<String>,
    },

    /// Run tests to verify the active outcome
    Verify,

    /// Advance to the next outcome (requires current outcome to be verified)
    Advance,

    /// Show project status dashboard
    Status,

    /// Show the audit ledger of events
    Trace {
        /// Show only the last N events
        #[arg(long, short)]
        limit: Option<usize>,
    },

    /// Run the specrail MCP server over stdio
    #[command(visible_alias = "mcpserver")]
    McpServer,
}

// ── Feature sub-commands ──────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum FeatureCommands {
    /// Create a new feature
    New {
        /// Unique feature identifier (e.g. auth-login)
        id: String,
        /// Component ID this feature belongs to (defaults to the active/default component)
        #[arg(long)]
        component: Option<String>,
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

    /// Edit an existing feature
    Edit {
        /// Feature ID
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

    /// Mark a feature as the active feature
    Activate {
        /// Feature ID
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum SolutionCommands {
    New {
        id: String,
        #[arg(long, short)]
        title: String,
        #[arg(long, short)]
        purpose: String,
    },
    List,
    Show {
        id: String,
    },
    Edit {
        id: String,
        #[arg(long, short)]
        title: String,
        #[arg(long, short)]
        purpose: String,
    },
    Activate {
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    New {
        solution_id: String,
        id: String,
        #[arg(long, short)]
        title: String,
        #[arg(long, short)]
        purpose: String,
    },
    List {
        solution_id: String,
    },
    Show {
        id: String,
    },
    Edit {
        id: String,
        #[arg(long, short)]
        title: String,
        #[arg(long, short)]
        purpose: String,
    },
    Activate {
        id: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ComponentCommands {
    New {
        project_id: String,
        id: String,
        #[arg(long, short)]
        title: String,
        #[arg(long, short)]
        purpose: String,
    },
    List {
        project_id: String,
    },
    Show {
        id: String,
    },
    Edit {
        id: String,
        #[arg(long, short)]
        title: String,
        #[arg(long, short)]
        purpose: String,
    },
    Activate {
        id: String,
    },
}

// ── Outcome sub-commands ──────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum OutcomeCommands {
    /// Create a new outcome for a feature
    ///
    /// An outcome is one verifiable step inside a feature. Outcomes are
    /// implemented and verified in `--order` sequence via `specrail advance`.
    ///
    /// After creation, register tests with `specrail test add` (or seed test
    /// paths with `--test` and generate them with `specrail test generate`),
    /// then activate the outcome with `specrail outcome activate`.
    New {
        /// Feature ID this outcome belongs to
        feature_id: String,
        /// Unique outcome identifier within the feature (e.g. outcome-1-domain)
        outcome_id: String,
        /// Short human-readable title (e.g. "Domain Validation")
        #[arg(long, short)]
        title: String,
        /// Acceptance criterion — what "done" looks like for this outcome
        /// (e.g. "User can log in with email and password and receives a JWT")
        #[arg(long, short)]
        goal: String,
        /// Execution sequence position (1 = first); `specrail advance` follows
        /// this numeric order across all outcomes in the feature
        #[arg(long, short)]
        order: u32,
        /// Outcome ID that must be verified before this one can be activated
        /// (repeatable; use the outcome's `outcome_id` value)
        #[arg(long = "prereq")]
        prerequisites: Vec<String>,
        /// Glob path the AI agent may modify when implementing this outcome
        /// (repeatable; omit to allow all paths — e.g. "src/auth/**")
        #[arg(long = "allow")]
        allowed_paths: Vec<String>,
        /// Glob path the AI agent must NOT touch (repeatable;
        /// e.g. "src/billing/**" to keep unrelated modules off-limits)
        #[arg(long = "forbid")]
        forbidden_paths: Vec<String>,
        /// Required manifest test ID that must pass
        #[arg(long = "test")]
        required_tests: Vec<String>,
        /// Test file path for AI-assisted generation via `specrail test generate`
        #[arg(long = "test-file")]
        required_test_files: Vec<String>,
    },

    /// List outcomes for a feature
    List {
        /// Feature ID
        feature_id: String,
    },

    /// Show details of an outcome
    Show {
        /// Feature ID
        feature_id: String,
        /// Outcome ID
        outcome_id: String,
    },

    /// Edit an existing outcome
    ///
    /// Re-specifying `--allow`, `--forbid`, or `--test` replaces the
    /// previously stored list entirely (omit the flag to clear the list).
    Edit {
        /// Feature ID
        feature_id: String,
        /// Outcome ID
        outcome_id: String,
        /// Short human-readable title
        #[arg(long, short)]
        title: String,
        /// Acceptance criterion — what "done" looks like for this outcome
        #[arg(long, short)]
        goal: String,
        /// Execution sequence position; `specrail advance` follows this order
        #[arg(long, short)]
        order: u32,
        /// Outcome ID that must be verified before this one (repeatable)
        #[arg(long = "prereq")]
        prerequisites: Vec<String>,
        /// Glob path the AI agent may modify (repeatable; omit to allow all)
        #[arg(long = "allow")]
        allowed_paths: Vec<String>,
        /// Glob path the AI agent must NOT touch (repeatable)
        #[arg(long = "forbid")]
        forbidden_paths: Vec<String>,
        /// Required manifest test ID that must pass
        #[arg(long = "test")]
        required_tests: Vec<String>,
        /// Test file path for AI-assisted generation via `specrail test generate`
        #[arg(long = "test-file")]
        required_test_files: Vec<String>,
    },

    /// Mark an outcome as the active outcome
    Activate {
        /// Feature ID
        feature_id: String,
        /// Outcome ID
        outcome_id: String,
    },
}

// ── Test sub-commands ─────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum TestCommands {
    /// Register a test in the manifest
    Add {
        /// Unique test identifier
        id: String,
        /// Test case or method name (defaults to id)
        #[arg(long)]
        name: Option<String>,
        /// Feature this test belongs to
        #[arg(long, short = 'f')]
        feature: String,
        /// Outcome this test belongs to
        #[arg(long, short = 'O')]
        outcome: String,
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

    /// Generate required tests from feature and outcome YAML using an AI agent
    Generate {
        /// Override the agent used for generation (defaults to `copilot`)
        #[arg(long, short)]
        agent: Option<String>,
    },

    /// Preview suggested required tests from feature and outcome YAML without writing files
    Suggest {
        /// Filter to a single feature
        #[arg(long, short)]
        feature: Option<String>,
        /// Filter to a single outcome within the selected feature
        #[arg(long, short = 'o')]
        outcome: Option<String>,
        /// Override the agent used for previewing suggestions (defaults to `copilot`)
        #[arg(long, short)]
        agent: Option<String>,
    },

    /// List tests in the manifest
    List {
        /// Filter by feature ID
        #[arg(long, short)]
        feature: Option<String>,
        /// Filter by outcome ID
        #[arg(long, short = 'o')]
        outcome: Option<String>,
    },

    /// Update the status of a test
    SetStatus {
        /// Test ID
        id: String,
        /// New status: planned | written | passing | failing
        status: String,
    },
}
