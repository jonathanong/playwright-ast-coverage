/// Returns true if `reference` matches `defined_pattern`.
///
/// Reference preprocessing:
/// - strip query and fragment,
/// - strip trailing slash unless the reference is `/`.
///
/// Pattern segments beginning with `:` match one segment. A final `*` segment
/// matches one or more segments. A final `**` segment matches zero or more
/// segments.
pub fn matches(reference: &str, defined_pattern: &str) -> bool {
    let ref_path = reference
        .split('?')
        .next()
        .unwrap_or(reference)
        .split('#')
        .next()
        .unwrap_or(reference);

    let ref_path = if ref_path.len() > 1 && ref_path.ends_with('/') {
        &ref_path[..ref_path.len() - 1]
    } else {
        ref_path
    };

    let ref_segs = segments(ref_path);
    let def_segs = segments(defined_pattern);

    for (index, def_seg) in def_segs.iter().enumerate() {
        let is_last = index + 1 == def_segs.len();
        if *def_seg == "**" && is_last {
            return ref_segs[index..].iter().all(|segment| !segment.is_empty());
        }

        if *def_seg == "*" && is_last {
            return ref_segs.len() > index
                && ref_segs[index..].iter().all(|segment| !segment.is_empty());
        }

        let Some(ref_seg) = ref_segs.get(index) else {
            return false;
        };
        if !segment_matches(ref_seg, def_seg) {
            return false;
        }
    }

    ref_segs.len() == def_segs.len()
}

fn segments(path: &str) -> Vec<&str> {
    if path == "/" || path.is_empty() {
        Vec::new()
    } else {
        path.strip_prefix('/').unwrap_or(path).split('/').collect()
    }
}

fn segment_matches(reference: &str, defined_pattern: &str) -> bool {
    if reference.is_empty() {
        return false;
    }
    if defined_pattern.starts_with(':') || defined_pattern == "*" {
        return true;
    }
    reference == defined_pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(matches("/api/v1/users", "/api/v1/users"));
    }

    #[test]
    fn param_match() {
        assert!(matches("/api/v1/users/42", "/api/v1/users/:id"));
    }

    #[test]
    fn wildcard_match() {
        assert!(matches("/api/v1/anything", "/api/v1/*"));
    }

    #[test]
    fn length_mismatch() {
        assert!(!matches("/api/v1", "/api/v1/users"));
    }

    #[test]
    fn literal_mismatch() {
        assert!(!matches("/api/v1/users", "/api/v1/posts"));
    }

    #[test]
    fn query_stripped() {
        assert!(matches("/api/v1/users?foo=bar", "/api/v1/users"));
    }

    #[test]
    fn fragment_stripped() {
        assert!(matches("/api/v1/users#section", "/api/v1/users"));
    }

    #[test]
    fn trailing_slash_stripped() {
        assert!(matches("/api/v1/users/", "/api/v1/users"));
    }

    #[test]
    fn root_slash_preserved() {
        assert!(matches("/", "/"));
    }
}
