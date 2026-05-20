use super::{
    helpers::{method_name, mounted_binding, object_identifier},
    ServerRouteVisitor, VERBS,
};
use crate::server_routes::model::{Binding, MountSite, RouteSite};
use crate::server_routes::source::line_number;
use crate::server_routes::types::Framework;
use oxc_ast::ast::{CallExpression, Expression};

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
        if self.client_http_from_expr(object) {
            return;
        }
        if let Some((binding, path)) = self.path_from_chain(object) {
            self.push_route(call, &binding, method, &path);
            return;
        }
        let Some(binding) = object_identifier(object) else {
            return;
        };
        let mut paths = self.route_args(&call.arguments, self.is_koa_router(&binding));
        if paths.is_empty()
            && self
                .facts
                .bindings
                .get(&binding)
                .is_some_and(|binding| !binding.prefixes.is_empty())
        {
            paths.push("/".to_string());
        }
        for path in paths {
            self.push_route(call, &binding, method, &path);
        }
    }

    fn record_hono_on(&mut self, call: &CallExpression<'_>, object: &Expression<'_>) {
        let Some(binding) = object_identifier(object) else {
            return;
        };
        let methods = match call.arguments.first() {
            Some(arg) => self.literal_args(arg),
            None => return,
        };
        let paths = match call.arguments.get(1) {
            Some(arg) => self.literal_args(arg),
            None => return,
        };
        for method in methods {
            for path in &paths {
                self.push_route(call, &binding, &method_name(&method), path);
            }
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
        let (prefix, child_args) = if method == "use" {
            match call.arguments.first().and_then(|arg| self.literal_arg(arg)) {
                Some(prefix) => (prefix, &call.arguments[1..]),
                None => ("/".to_string(), call.arguments.as_slice()),
            }
        } else {
            let Some(prefix) = call.arguments.first().and_then(|arg| self.literal_arg(arg)) else {
                return;
            };
            (prefix, &call.arguments[1..])
        };
        for child in child_args.iter().filter_map(mounted_binding) {
            self.facts.mounts.push(MountSite {
                parent: parent.clone(),
                child,
                prefix: prefix.clone(),
            });
        }
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
            return self
                .facts
                .bindings
                .get(id.name.as_str())
                .filter(|binding| binding.framework == Framework::KoaPathMatch)
                .and_then(|_| call.arguments.first().and_then(|arg| self.literal_arg(arg)))
                .map(|path| (id.name.to_string(), path));
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
}
