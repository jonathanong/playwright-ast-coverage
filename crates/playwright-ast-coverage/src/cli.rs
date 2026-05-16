use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Clone)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(long, default_value = ".", global = true)]
    pub root: PathBuf,

    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(long, global = true)]
    pub playwright_config: Vec<PathBuf>,

    #[arg(long, global = true)]
    pub project: Option<String>,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub assert_conditional_tests: bool,

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
    Check,
    Edges,
    Related {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
    },
    Tests {
        #[arg(num_args = 0..)]
        files: Vec<PathBuf>,
    },
}
