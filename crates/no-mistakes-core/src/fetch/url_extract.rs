use crate::ast;
use crate::fetch::types::UrlExtraction;
use oxc_ast::ast::Argument;
use oxc_span::GetSpan;

pub fn extract_url_from_argument(arg: &Argument, source: &str) -> UrlExtraction {
    match arg {
        Argument::StringLiteral(s) => UrlExtraction {
            path: s.value.to_string(),
            raw_path: s.value.to_string(),
            is_dynamic: false,
            is_unsupported: false,
        },
        Argument::TemplateLiteral(t) => {
            let is_dynamic = !t.expressions.is_empty();
            UrlExtraction {
                path: ast::template_literal_text(t, source),
                raw_path: source_text(t.span().start as usize, t.span().end as usize, source)
                    .unwrap_or_else(|| "dynamic".to_string()),
                is_dynamic,
                is_unsupported: is_dynamic,
            }
        }
        _ => UrlExtraction {
            path: "dynamic".to_string(),
            raw_path: source_text(arg.span().start as usize, arg.span().end as usize, source)
                .unwrap_or_else(|| "dynamic".to_string()),
            is_dynamic: true,
            is_unsupported: true,
        },
    }
}

pub fn source_text(start: usize, end: usize, source: &str) -> Option<String> {
    if start > end || end > source.len() {
        return None;
    }

    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return None;
    }

    Some(source[start..end].trim().to_string())
}
