use no_mistakes_core::routes as core_routes;
use std::path::Path;

pub use core_routes::Route;

const PAGE_STEMS: &[&str] = &["page"];

pub fn collect_routes(frontend_root: &Path) -> Vec<Route> {
    core_routes::collect_routes(frontend_root, PAGE_STEMS)
}
