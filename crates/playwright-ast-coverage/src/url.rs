use crate::matcher;

pub(crate) fn normalize_url(raw: &str, base_urls: &[String]) -> Option<String> {
    if raw.starts_with("//") {
        return None;
    }

    if raw.starts_with('/') {
        return Some(raw.to_string());
    }

    for base_url in base_urls {
        let base = base_url.trim_end_matches('/');
        if let Some(rest) = raw.strip_prefix(base) {
            if rest.is_empty() {
                return Some("/".to_string());
            }
            if rest.starts_with('/') {
                return Some(rest.to_string());
            }
        }
    }

    None
}

pub(crate) fn is_dynamic_pattern_segment(segment: &str) -> bool {
    segment.starts_with(':') || segment == "*" || segment == "**"
}

pub(crate) fn is_ignored(route: &str, ignored: &[String]) -> bool {
    ignored
        .iter()
        .any(|pattern| route == pattern || matcher::matches(route, pattern))
}
