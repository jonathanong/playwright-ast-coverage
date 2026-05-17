pub mod discover;
pub mod legacy;
pub mod schema;
pub mod view;

pub use discover::{find_config_root, load_v2_config};
pub use schema::NoMistakesConfig;
pub use view::ConfigView;

/// Identifies which per-tool legacy config format was found on disk.
#[derive(Clone, Copy, Debug)]
pub enum ToolKind {
    Playwright,
    ReactTraits,
    NextToFetch,
}

#[cfg(test)]
mod tests;
