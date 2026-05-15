use crate::ast;

pub(super) fn callee_is_static_member_named(
    callee: &oxc_ast::ast::Expression<'_>,
    method: &str,
) -> bool {
    callee_static_member_name(callee).is_some_and(|name| name == method)
}

pub(super) fn callee_static_member_name<'a>(
    callee: &'a oxc_ast::ast::Expression<'a>,
) -> Option<&'a str> {
    match callee {
        oxc_ast::ast::Expression::StaticMemberExpression(member) => {
            Some(member.property.name.as_str())
        }
        oxc_ast::ast::Expression::ParenthesizedExpression(parenthesized) => {
            callee_static_member_name(&parenthesized.expression)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub(super) enum SelectorArgumentMode {
    First,
    All,
}

pub(super) fn selector_argument_mode(
    callee: &oxc_ast::ast::Expression<'_>,
) -> Option<SelectorArgumentMode> {
    match callee_static_member_name(callee)? {
        "dragAndDrop" => Some(SelectorArgumentMode::All),
        "$" | "$$" | "$$eval" | "$eval" | "check" | "click" | "dblclick" | "dispatchEvent"
        | "dragTo" | "evalOnSelector" | "evalOnSelectorAll" | "fill" | "focus" | "frameLocator"
        | "getAttribute" | "hover" | "innerHTML" | "innerText" | "inputValue" | "isChecked"
        | "isDisabled" | "isEditable" | "isEnabled" | "isHidden" | "isVisible" | "locator"
        | "press" | "selectOption" | "setChecked" | "tap" | "textContent" | "type" | "uncheck"
        | "waitForSelector" => Some(SelectorArgumentMode::First),
        _ => None,
    }
}

pub(super) fn selector_argument_literals(
    call: &oxc_ast::ast::CallExpression<'_>,
    source: &str,
    mode: SelectorArgumentMode,
) -> Vec<String> {
    call.arguments
        .iter()
        .enumerate()
        .filter(|(index, _)| matches!(mode, SelectorArgumentMode::All) || *index == 0)
        .filter_map(|(_, argument)| match argument {
            oxc_ast::ast::Argument::StringLiteral(literal) => Some(literal.value.to_string()),
            oxc_ast::ast::Argument::TemplateLiteral(template) => {
                Some(ast::template_literal_text(template.as_ref(), source))
            }
            _ => None,
        })
        .collect()
}

pub(super) fn extract_get_by_test_id_call(
    call: &oxc_ast::ast::CallExpression<'_>,
    source: &str,
    attributes: &[String],
    insert: &mut impl FnMut(super::types::PlaywrightSelector),
) {
    use super::types::SelectorMatcher;

    let Some(argument) = call.arguments.first() else {
        return;
    };

    let matcher = match argument {
        oxc_ast::ast::Argument::StringLiteral(literal) => Some((
            literal.value.to_string(),
            SelectorMatcher::Exact(literal.value.to_string()),
        )),
        oxc_ast::ast::Argument::TemplateLiteral(template) => {
            let value = ast::template_literal_text(template, source);
            Some((value.clone(), SelectorMatcher::Exact(value)))
        }
        oxc_ast::ast::Argument::RegExpLiteral(regex) => {
            let value = regex.regex.pattern.text.to_string();
            let compiled = regex::Regex::new(&value).ok();
            Some((
                format!("/{value}/"),
                SelectorMatcher::Regex {
                    pattern: value,
                    compiled,
                },
            ))
        }
        _ => None,
    };

    let Some((display, matcher)) = matcher else {
        return;
    };
    for attribute in attributes {
        insert(super::types::PlaywrightSelector {
            attribute: attribute.clone(),
            selector: format!("getByTestId({display})"),
            matcher: matcher.clone(),
        });
    }
}
