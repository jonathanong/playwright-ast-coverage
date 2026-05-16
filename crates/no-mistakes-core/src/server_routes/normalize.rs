pub(crate) fn join_paths(prefix: &str, path: &str) -> String {
    if prefix.is_empty() || prefix == "/" {
        return clean_path(path);
    }
    if path.is_empty() || path == "/" {
        return clean_path(prefix);
    }
    clean_path(&format!(
        "{}/{}",
        prefix.trim_end_matches('/'),
        path.trim_start_matches('/')
    ))
}

pub(crate) fn normalize_route(path: &str) -> String {
    let clean = clean_path(path);
    let mut out = Vec::new();
    let mut in_braced_wildcard = false;
    for segment in clean.trim_start_matches('/').split('/') {
        if segment.is_empty() {
            continue;
        }
        if in_braced_wildcard {
            if segment.ends_with('}') {
                in_braced_wildcard = false;
            }
            continue;
        }
        if segment.starts_with('{') && !segment.ends_with('}') {
            in_braced_wildcard = true;
            out.push("**".to_string());
        } else if segment.contains('*') && segment.starts_with('{') {
            out.push("**".to_string());
        } else if segment == "*" || segment.starts_with(':') {
            out.push("*".to_string());
        } else {
            out.push(segment.to_string());
        }
    }
    if out.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", out.join("/"))
    }
}

fn clean_path(path: &str) -> String {
    let path = path.split(['?', '#']).next().unwrap_or(path).trim();
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    if path.len() > 1 {
        path.trim_end_matches('/').to_string()
    } else {
        path
    }
}

#[cfg(test)]
mod tests;
