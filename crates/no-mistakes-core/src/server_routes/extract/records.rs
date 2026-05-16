use super::{
    helpers::{method_name, mounted_binding, object_identifier},
    ServerRouteVisitor, VERBS,
};
use crate::server_routes::model::{Binding, MountSite, RouteSite};
use crate::server_routes::source::line_number;
use crate::server_routes::types::Framework;
use oxc_ast::ast::{Argument, CallExpression, Expression};

impl ServerRouteVisitor<'_> {
    pub(super) fn record_call(&mut self, call: &CallExpression<'_>) {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let method = member.property.name.as_str();
        if method == "use" || method == "route" {
            self.record_mount_or_api_route(call, &member.object, method);
        } else if method == "prefix" || method == "basePath" {
            self.record_prefix(call, &member.object);
        } else if method == "on" {
            self.record_hono_on(call, &member.object);
        } else if VERBS.contains(&method) {
            self.record_verb(call, &member.object, method);
        }
    }

    fn record_verb(&mut self, call: &CallExpression<'_>, object: &Expression<'_>, method: &str) {
        if let Some((binding, path)) = self.path_from_chain(object) {
            self.push_route(call, &binding, method, &path);
            return;
        }
        let Some(binding) = object_identifier(object) else {
            return;
        };
        for path in route_args(
            &call.arguments,
            method == "all" || self.is_koa_router(&binding),
        ) {
            self.push_route(call, &binding, method, &path);
        }
    }

    fn record_hono_on(&mut self, call: &CallExpression<'_>, object: &Expression<'_>) {
        let Some(binding) = object_identifier(object) else {
            return;
        };
        let Some(method) = call.arguments.first().and_then(|arg| self.literal_arg(arg)) else {
            return;
        };
        if let Some(path) = call.arguments.get(1).and_then(|arg| self.literal_arg(arg)) {
            self.push_route(call, &binding, &method.to_lowercase(), &path);
        }
    }

    fn record_mount_or_api_route(
        &mut self,
        call: &CallExpression<'_>,
        object: &Expression<'_>,
        method: &str,
    ) {
        let Some(parent) = object_identifier(object) else {
            return;
        };
        if method == "route" && call.arguments.get(1).and_then(mounted_binding).is_none() {
            return;
        }
        let Some(prefix) = call.arguments.first().and_then(|arg| self.literal_arg(arg)) else {
            return;
        };
        let Some(child) = call.arguments.get(1).and_then(mounted_binding) else {
            return;
        };
        self.facts.mounts.push(MountSite {
            parent,
            child,
            prefix,
        });
    }

    fn record_prefix(&mut self, call: &CallExpression<'_>, object: &Expression<'_>) {
        let Some(binding) = object_identifier(object) else {
            return;
        };
        let Some(prefix) = call.arguments.first().and_then(|arg| self.literal_arg(arg)) else {
            return;
        };
        self.facts
            .bindings
            .entry(binding)
            .or_insert_with(|| Binding::new(Framework::Heuristic, None))
            .prefixes
            .push(prefix);
    }

    fn path_from_chain(&self, object: &Expression<'_>) -> Option<(String, String)> {
        let Expression::CallExpression(call) = object else {
            return None;
        };
        if let Expression::Identifier(id) = &call.callee {
            if self
                .facts
                .bindings
                .get(id.name.as_str())
                .is_some_and(|binding| binding.framework == Framework::KoaPathMatch)
            {
                let path = call
                    .arguments
                    .first()
                    .and_then(|arg| self.literal_arg(arg))?;
                return Some((id.name.to_string(), path));
            }
        }
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return None;
        };
        if member.property.name.as_str() == "route" || member.property.name.as_str() == "basePath" {
            let binding = object_identifier(&member.object)?;
            let path = call
                .arguments
                .first()
                .and_then(|arg| self.literal_arg(arg))?;
            return Some((binding, path));
        }
        self.path_from_chain(&member.object)
    }

    fn push_route(&mut self, call: &CallExpression<'_>, binding: &str, method: &str, path: &str) {
        let framework = self.framework_for(binding);
        self.facts.routes.push(RouteSite {
            file: self.path.to_path_buf(),
            line: line_number(self.source, call.span.start),
            binding: binding.to_string(),
            method: method_name(method),
            raw_path: path.to_string(),
            path: path.to_string(),
            framework,
        });
    }

    fn framework_for(&self, binding: &str) -> Framework {
        self.facts
            .bindings
            .get(binding)
            .map(|binding| binding.framework)
            .unwrap_or(Framework::Heuristic)
    }

    fn is_koa_router(&self, binding: &str) -> bool {
        self.facts
            .bindings
            .get(binding)
            .is_some_and(|binding| binding.framework == Framework::KoaRouter)
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
}

fn route_args(args: &[Argument<'_>], allow_named: bool) -> Vec<String> {
    if allow_named {
        if let (Some(Argument::StringLiteral(first)), Some(Argument::StringLiteral(second))) =
            (args.first(), args.get(1))
        {
            if !first.value.starts_with('/') && second.value.starts_with('/') {
                return vec![second.value.as_str().to_string()];
            }
        }
    }
    let mut paths = Vec::new();
    for arg in args.iter().take(2) {
        match arg {
            Argument::StringLiteral(value) if value.value.starts_with('/') => {
                paths.push(value.value.as_str().to_string());
            }
            Argument::ArrayExpression(array) => {
                for element in &array.elements {
                    if let oxc_ast::ast::ArrayExpressionElement::StringLiteral(value) = element {
                        paths.push(value.value.as_str().to_string());
                    }
                }
            }
            Argument::StringLiteral(value) if allow_named && paths.is_empty() => {
                paths.push(value.value.as_str().to_string());
            }
            _ => {}
        }
    }
    paths
}
