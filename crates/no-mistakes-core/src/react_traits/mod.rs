pub(crate) mod analyze;
pub(crate) mod pipeline;
pub(crate) mod report;
pub(crate) mod traits;

pub use pipeline::run_analyze;
pub use pipeline::run_check;
pub use report::text::{print_results, print_violations};
pub use report::types::{AggregatedFacts, ComponentFacts, Violation};
