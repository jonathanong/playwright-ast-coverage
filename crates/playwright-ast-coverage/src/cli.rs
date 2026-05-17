use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Clone)]
#[command(author, version, about)]
pub struct Cli {
    /// Repository or package root to analyze.
    #[arg(long, default_value = ".", global = true)]
    pub root: PathBuf,

    /// Analyzer config file. Relative paths are resolved from --root.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Playwright config file. May be repeated and overrides analyzer config.
    #[arg(long, global = true)]
    pub playwright_config: Vec<PathBuf>,

    /// Filter by top-level Playwright config name.
    #[arg(long, global = true)]
    pub project: Option<String>,

    /// Emit JSON instead of text output.
    #[arg(long, global = true)]
    pub json: bool,

    /// Require coverage from active tests, not conditional tests.
    #[arg(long, global = true)]
    pub assert_conditional_tests: bool,

    /// Allow skipped Playwright tests and suites to count as coverage.
    #[arg(long, global = true)]
    pub allow_skipped_tests: bool,

    #[arg(
        long,
        global = true,
        help = "Fail check when exact test ID values are used more than once"
    )]
    pub assert_unique_test_ids: bool,

    #[arg(
        long,
        global = true,
        help = "Fail check when exact HTML id values are used more than once"
    )]
    pub assert_unique_html_ids: bool,

    #[arg(
        long,
        global = true,
        help = "Deprecated: use --assert-unique-test-ids and --assert-unique-html-ids"
    )]
    pub assert_unique_selectors: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    /// Check route and selector coverage.
    Check,
    /// Print test-to-route and test-to-selector edges.
    Edges,
    /// Print Playwright tests that cover the given route/component files.
    Related {
        /// Route or selector source files to look up.
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
    },
    /// Print route, selector, and fetch assertions grouped by test.
    Tests {
        /// Optional Playwright test files to include. Omit to list all discovered tests.
        #[arg(num_args = 0..)]
        files: Vec<PathBuf>,
    },
}
