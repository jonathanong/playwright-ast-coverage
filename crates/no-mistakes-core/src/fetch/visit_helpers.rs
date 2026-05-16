use crate::fetch::cache_opts::{
    cache_wrapper_name, extract_fetch_cache_options, infer_cached_wrapper_name,
};
use crate::fetch::types::{CacheKind, FetchOccurrence, FetchSide};
use crate::fetch::url_extract::extract_url_from_argument;
use crate::fetch::visitor::FetchVisitor;
use oxc_ast::ast::{Argument, CallExpression, Expression};
use oxc_span::GetSpan;

pub fn try_extract_fetch<'a>(
    expr: &CallExpression<'a>,
    visitor: &FetchVisitor<'a>,
) -> Option<FetchOccurrence> {
    let Expression::Identifier(ident) = &expr.callee else {
        return None;
    };
    if ident.name.as_ref() != "fetch" || visitor.is_fetch_shadowed() {
        return None;
    }

    let mut method = "GET".to_string();
    let mut cached = false;
    let mut cache_kind = CacheKind::None;
    let line = visitor.source[..expr.span().start as usize].lines().count() + 1;

    let (path, raw_path, is_dynamic, is_unsupported) = if let Some(arg) = expr.arguments.first() {
        let result = extract_url_from_argument(arg, visitor.source);
        (
            result.path,
            result.raw_path,
            result.is_dynamic,
            result.is_unsupported,
        )
    } else {
        ("unknown".to_string(), "unknown".to_string(), true, true)
    };

    if let Some(ck) = &visitor.cached_kind {
        cached = true;
        cache_kind = ck.clone();
    }

    if let Some(Argument::ObjectExpression(obj)) = expr.arguments.get(1) {
        for prop in &obj.properties {
            if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                if let Some(name) = p.key.static_name() {
                    if name.as_ref() == "method" {
                        if let Expression::StringLiteral(s) = &p.value {
                            method = s.value.to_string();
                        }
                    }
                }
            }
        }
        let (seen_cached, seen_cache_kind) = extract_fetch_cache_options(obj);
        if !cached {
            cached = seen_cached;
            cache_kind = seen_cache_kind;
        }
    }

    let side = if visitor.is_client {
        FetchSide::Client
    } else {
        FetchSide::Server
    };

    Some(FetchOccurrence {
        path,
        raw_path,
        method,
        file: visitor.file.clone(),
        line,
        side,
        rsc: !visitor.is_client && !visitor.is_route_handler,
        cached,
        cache_kind,
        cached_function: visitor.cached_function.clone(),
        dynamic: is_dynamic,
        unsupported: is_unsupported,
    })
}

pub fn enter_cache_wrapper<'a>(
    expr: &CallExpression<'a>,
    visitor: &mut FetchVisitor<'a>,
) -> (Option<String>, Option<CacheKind>) {
    let (wrapper_name, cached_kind) = cache_wrapper_name(expr).unwrap();
    let previous_cached_function = visitor.cached_function.clone();
    let previous_cached_kind = visitor.cached_kind.clone();
    visitor.cached_function =
        infer_cached_wrapper_name(visitor.source, expr).or(Some(wrapper_name));
    visitor.cached_kind = Some(cached_kind);
    (previous_cached_function, previous_cached_kind)
}

pub fn leave_cache_wrapper(
    visitor: &mut FetchVisitor<'_>,
    previous_cached_function: Option<String>,
    previous_cached_kind: Option<CacheKind>,
) {
    visitor.cached_function = previous_cached_function;
    visitor.cached_kind = previous_cached_kind;
}
