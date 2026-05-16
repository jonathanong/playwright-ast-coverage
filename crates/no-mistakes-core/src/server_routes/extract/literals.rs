use super::ServerRouteVisitor;
use oxc_ast::ast::{Argument, ArrayExpressionElement};

impl ServerRouteVisitor<'_> {
    pub(super) fn route_args(&self, args: &[Argument<'_>], allow_named: bool) -> Vec<String> {
        if args.is_empty() {
            return Vec::new();
        }
        if allow_named && args.len() >= 2 {
            let first = self.literal_arg(&args[0]);
            let second = self.literal_args(&args[1]);
            if first.is_some_and(|value| !value.starts_with('/')) {
                let named_paths = second
                    .into_iter()
                    .filter(|value| value.starts_with('/'))
                    .collect::<Vec<_>>();
                if !named_paths.is_empty() {
                    return named_paths;
                }
            }
        }
        let Some(first) = args.first() else {
            return Vec::new();
        };
        self.literal_args(first)
            .into_iter()
            .filter(|path| path.starts_with('/') || !allow_named)
            .collect()
    }

    pub(super) fn literal_arg(&self, arg: &Argument<'_>) -> Option<String> {
        match arg {
            Argument::StringLiteral(value) => Some(value.value.as_str().to_string()),
            Argument::TemplateLiteral(template) if template.expressions.is_empty() => Some(
                template
                    .quasis
                    .iter()
                    .filter_map(|quasi| quasi.value.cooked.as_deref())
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            Argument::Identifier(id) => self.const_strings.get(id.name.as_str()).cloned(),
            _ => None,
        }
    }

    pub(super) fn literal_args(&self, arg: &Argument<'_>) -> Vec<String> {
        match arg {
            Argument::ArrayExpression(array) => array
                .elements
                .iter()
                .filter_map(|element| self.literal_element(element))
                .collect(),
            _ => self.literal_arg(arg).into_iter().collect(),
        }
    }

    fn literal_element(&self, element: &ArrayExpressionElement<'_>) -> Option<String> {
        match element {
            ArrayExpressionElement::StringLiteral(value) => Some(value.value.as_str().to_string()),
            ArrayExpressionElement::TemplateLiteral(template)
                if template.expressions.is_empty() =>
            {
                Some(
                    template
                        .quasis
                        .iter()
                        .filter_map(|quasi| quasi.value.cooked.as_deref())
                        .collect::<Vec<_>>()
                        .join(""),
                )
            }
            ArrayExpressionElement::Identifier(id) => {
                self.const_strings.get(id.name.as_str()).cloned()
            }
            _ => None,
        }
    }
}
