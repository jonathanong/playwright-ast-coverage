/// Returns `true` if `reference` matches `defined_pattern`.
///
/// Preprocessing for `reference`:
/// - Strip trailing `/` (unless it IS `/`)
/// - Split at `?` and `#` and take the first part
///
/// Segment comparison:
/// - If `defined_pattern` segment starts with `:` → matches any single non-empty segment
///   (including `:param` substituted references)
/// - If `defined_pattern` segment is `*` → matches one non-empty segment, or one or more
///   remaining segments when it is the final pattern segment
/// - If `defined_pattern` segment is `**` → matches zero or more remaining segments
/// - Otherwise: exact string equality (case-sensitive)
pub fn matches(reference: &str, defined_pattern: &str) -> bool {
    // Strip query and fragment from reference
    let ref_path = reference
        .split('?')
        .next()
        .unwrap_or(reference)
        .split('#')
        .next()
        .unwrap_or(reference);

    // Strip trailing slash (unless it is the root "/")
    let ref_path = if ref_path.len() > 1 && ref_path.ends_with('/') {
        &ref_path[..ref_path.len() - 1]
    } else {
        ref_path
    };

    let ref_segs: Vec<&str> = ref_path.split('/').collect();
    let def_segs: Vec<&str> = defined_pattern.split('/').collect();

    matches_segments(&ref_segs, &def_segs)
}

/// Returns `true` if `reference` matches any pattern in `defined_patterns`.
pub fn matches_any(reference: &str, defined_patterns: &[String]) -> bool {
    defined_patterns.iter().any(|p| matches(reference, p))
}

fn matches_segments(reference: &[&str], defined: &[&str]) -> bool {
    match (reference, defined) {
        ([], []) => true,
        (_, []) => false,
        ([], [next, rest @ ..]) => *next == "**" && rest.is_empty(),
        ([ref_head, ref_rest @ ..], [def_head, def_rest @ ..])
            if def_head.starts_with(':') && !ref_head.is_empty() =>
        {
            matches_segments(ref_rest, def_rest)
        }
        ([ref_head, ref_rest @ ..], ["*", def_rest @ ..]) if !ref_head.is_empty() => {
            def_rest.is_empty() || matches_segments(ref_rest, def_rest)
        }
        (reference, ["**", def_rest @ ..]) => {
            def_rest.is_empty()
                || matches_segments(reference, def_rest)
                || (!reference.is_empty() && matches_segments(&reference[1..], defined))
        }
        ([ref_head, ref_rest @ ..], [def_head, def_rest @ ..]) if ref_head == def_head => {
            matches_segments(ref_rest, def_rest)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
