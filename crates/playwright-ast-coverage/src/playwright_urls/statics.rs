use crate::ast;
use oxc_ast::ast::CallExpression;
use std::collections::HashMap;

pub fn collect_static_zero_arg_paths(source: &str) -> HashMap<String, Vec<String>> {
    let pattern = regex::Regex::new(
        r#"([A-Za-z_$][\w$]*)\s*:\s*\(\s*\)\s*=>\s*(?:"([^"`]+)"|'([^'`]+)'|`([^'"`]+)`)"#,
    )
    .expect("static route helper regex should compile");
    let mut candidates: HashMap<String, (Vec<String>, usize)> = HashMap::new();
    for captures in pattern.captures_iter(source) {
        let full_match = captures.get(0).expect("full capture should exist");
        if !source_offset_is_code(source, full_match.start()) {
            continue;
        }
        if let Some((name, value)) = (|| {
            let name = captures.get(1)?;
            let value = captures
                .get(2)
                .or_else(|| captures.get(3))
                .or_else(|| captures.get(4))?;
            Some((name.as_str().to_string(), value.as_str().to_string()))
        })() {
            let entry = candidates.entry(name).or_default();
            entry.0.push(value);
            entry.1 += 1;
        }
    }
    candidates
        .into_iter()
        .filter_map(|(name, (values, count))| (count == 1).then_some((name, values)))
        .collect()
}

pub fn static_zero_arg_path_call(
    call: &CallExpression<'_>,
    static_zero_arg_paths: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if !call.arguments.is_empty() {
        return Vec::new();
    }
    let Some(path) = ast::expression_path(&call.callee) else {
        return Vec::new();
    };
    if path.len() != 1 {
        return Vec::new();
    }
    let name = &path[path.len() - 1];
    static_zero_arg_paths
        .get(name.as_str())
        .cloned()
        .unwrap_or_default()
}

pub fn source_offset_is_code(source: &str, offset: usize) -> bool {
    let mut chars = source.char_indices().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;

    while let Some((index, ch)) = chars.next() {
        if index >= offset {
            return !in_single
                && !in_double
                && !in_template
                && !in_line_comment
                && !in_block_comment;
        }

        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            if ch == '*' && chars.peek().is_some_and(|(_, next)| *next == '/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        if in_single || in_double || in_template {
            if ch == '\\' {
                escaped = true;
            } else if in_single {
                in_single = ch != '\'';
            } else if in_double {
                in_double = ch != '"';
            } else {
                in_template = ch != '`';
            }
            continue;
        }

        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            chars.next();
            in_line_comment = true;
        } else if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '*') {
            chars.next();
            in_block_comment = true;
        } else if ch == '\'' {
            in_single = true;
        } else if ch == '"' {
            in_double = true;
        } else if ch == '`' {
            in_template = true;
        }
    }

    !in_single && !in_double && !in_template && !in_line_comment && !in_block_comment
}
