/// Returns true if `reference` matches `defined_pattern`.
///
/// Reference preprocessing:
/// - strip query and fragment,
/// - strip trailing slash unless the reference is `/`.
///
/// Pattern segments beginning with `:` match one segment. A `*` segment also
/// matches one segment.
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

    let ref_segs: Vec<&str> = ref_path.split('/').collect();
    let def_segs: Vec<&str> = defined_pattern.split('/').collect();

    if ref_segs.len() != def_segs.len() {
        return false;
    }

    for (ref_seg, def_seg) in ref_segs.iter().zip(def_segs.iter()) {
        if def_seg.starts_with(':') || *def_seg == "*" {
            continue;
        }
        if ref_seg != def_seg {
            return false;
        }
    }

    true
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
