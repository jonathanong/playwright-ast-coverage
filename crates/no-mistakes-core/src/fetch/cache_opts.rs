use crate::fetch::types::CacheKind;
use oxc_ast::ast::{CallExpression, Expression};
use oxc_span::GetSpan;

pub fn cache_wrapper_name(expr: &CallExpression<'_>) -> Option<(String, CacheKind)> {
    let Expression::Identifier(identifier) = &expr.callee else {
        return None;
    };
    match identifier.name.as_ref() {
        "cache" => Some((identifier.name.to_string(), CacheKind::ReactCache)),
        "unstable_cache" => Some((identifier.name.to_string(), CacheKind::UnstableCache)),
        _ => None,
    }
}

pub fn extract_fetch_cache_options(obj: &oxc_ast::ast::ObjectExpression<'_>) -> (bool, CacheKind) {
    let mut cached = false;
    let mut cache_kind = CacheKind::None;

    for property in &obj.properties {
        let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        let Some(name) = property.key.static_name() else {
            continue;
        };

        match name.as_ref() {
            "cache" => {
                if let Expression::StringLiteral(value) = &property.value {
                    if value.value == "force-cache" {
                        cached = true;
                        cache_kind = CacheKind::FetchCache;
                    }
                }
            }
            "next" => {
                if let Expression::ObjectExpression(next_obj) = &property.value {
                    for next_property in &next_obj.properties {
                        let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(next_property) =
                            next_property
                        else {
                            continue;
                        };
                        let Some(next_name) = next_property.key.static_name() else {
                            continue;
                        };
                        match next_name.as_ref() {
                            "revalidate" => match &next_property.value {
                                Expression::NumericLiteral(value) if value.value > 0.0 => {
                                    cached = true;
                                    cache_kind = CacheKind::FetchNextRevalidate;
                                }
                                _ => {}
                            },
                            "tags" => {
                                cached = true;
                                cache_kind = CacheKind::FetchNextTags;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (cached, cache_kind)
}

pub fn infer_cached_wrapper_name(source: &str, expr: &CallExpression<'_>) -> Option<String> {
    let assignment = &source[..expr.span().start as usize];
    let assignment = assignment.trim_end();
    let equal_sign = assignment.rfind('=')?;
    if !assignment[equal_sign + 1..].trim().is_empty() {
        return None;
    }

    let lhs = assignment[..equal_sign].trim_end();

    let mut cursor = lhs.len();
    let end = cursor;
    while cursor > 0 {
        let ch = lhs[..cursor]
            .chars()
            .last()
            .expect("non-empty slice always has a last char");
        if is_identifier_char(ch) {
            cursor -= ch.len_utf8();
        } else {
            break;
        }
    }
    if cursor == end {
        return None;
    }

    let name = &lhs[cursor..end];
    if cursor > 0
        && lhs[..cursor]
            .chars()
            .last()
            .is_some_and(|ch| ch == '.' || ch == '?' || ch == ':' || ch == ')' || ch == ']')
    {
        return None;
    }
    if name
        .chars()
        .next()
        .is_some_and(|char| char.is_ascii_alphabetic() || char == '_' || char == '$')
    {
        Some(name.to_string())
    } else {
        None
    }
}

pub fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '$'
}
