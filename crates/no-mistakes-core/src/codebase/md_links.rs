/// Extract link targets from Markdown source using pulldown-cmark.
///
/// Returns raw link URL strings. Callers are responsible for filtering
/// external URLs and resolving relative paths to absolute file paths.
pub fn extract_links(source: &str) -> Vec<String> {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    let mut links = Vec::new();

    for event in Parser::new_ext(source, Options::all()) {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                links.push(dest_url.into_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                links.push(dest_url.into_string());
            }
            _ => {}
        }
    }

    links
}

/// Returns true if the URL is an external link that should not be resolved
/// to a local file path.
pub fn is_external(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("//")
        || url.starts_with('#')
        || url.starts_with('?')
}

#[cfg(test)]
mod tests;
