use super::{helpers::first_object_prefix, import_names, ServerRouteVisitor};
use crate::server_routes::model::{Binding, ImportBinding};
use crate::server_routes::types::Framework;
use oxc_ast::ast::{CallExpression, Expression, ImportDeclarationSpecifier};

impl ServerRouteVisitor<'_> {
    pub(super) fn record_import(
        &mut self,
        source: &str,
        specifier: &ImportDeclarationSpecifier<'_>,
    ) {
        let (local, imported) = import_names(specifier);
        match source {
            "express" if imported == "default" || imported == "Router" => {
                self.express_names.insert(local.clone());
            }
            "hono" | "@hono/hono" if imported == "Hono" => {
                self.hono_names.insert(local.clone());
            }
            "@koa/router" | "koa-router" if imported == "default" || imported == "Router" => {
                self.koa_router_names.insert(local.clone());
            }
            "koa-path-match" | "@koa/path-match" if imported == "default" => {
                self.path_match_names.insert(local.clone());
            }
            "@jongleberry/api-server" | "api-server" if imported == "createApp" => {
                self.api_server_names.insert(local.clone());
            }
            _ => {}
        }
        if is_client_http_module(source) {
            self.client_http_names.insert(local.clone());
        }
        self.facts.imports.push(ImportBinding {
            local,
            imported,
            source: source.to_string(),
        });
    }

    pub(super) fn binding_from_expr(&self, expr: &Expression<'_>) -> Option<Binding> {
        match expr {
            Expression::CallExpression(call) => self.call_binding(call),
            Expression::NewExpression(new_expr) => {
                let Expression::Identifier(id) = &new_expr.callee else {
                    return None;
                };
                let name = id.name.as_str();
                if self.hono_names.contains(name) {
                    Some(Binding::new(
                        Framework::Hono,
                        first_object_prefix(&new_expr.arguments),
                    ))
                } else if self.koa_router_names.contains(name) {
                    Some(Binding::new(
                        Framework::KoaRouter,
                        first_object_prefix(&new_expr.arguments),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn client_http_from_expr(&self, expr: &Expression<'_>) -> bool {
        match expr {
            Expression::Identifier(id) => self.client_http_names.contains(id.name.as_str()),
            Expression::ParenthesizedExpression(expr) => {
                self.client_http_from_expr(&expr.expression)
            }
            Expression::CallExpression(call) => self.client_http_from_call(call),
            Expression::StaticMemberExpression(member) => {
                self.client_http_from_expr(&member.object)
            }
            _ => false,
        }
    }

    fn client_http_from_call(&self, call: &CallExpression<'_>) -> bool {
        match &call.callee {
            Expression::Identifier(id) => self.client_http_names.contains(id.name.as_str()),
            Expression::StaticMemberExpression(member) => {
                self.client_http_from_expr(&member.object)
            }
            _ => false,
        }
    }

    fn call_binding(&self, call: &CallExpression<'_>) -> Option<Binding> {
        match &call.callee {
            Expression::Identifier(id) if self.express_names.contains(id.name.as_str()) => {
                Some(Binding::new(Framework::Express, None))
            }
            Expression::Identifier(id) if self.api_server_names.contains(id.name.as_str()) => {
                Some(Binding::new(Framework::ApiServer, None))
            }
            Expression::Identifier(id) if self.path_match_names.contains(id.name.as_str()) => {
                Some(Binding::new(Framework::KoaPathMatch, None))
            }
            Expression::StaticMemberExpression(member)
                if member.property.name.as_str() == "Router"
                    && matches!(&member.object, Expression::Identifier(id) if self.express_names.contains(id.name.as_str())) =>
            {
                Some(Binding::new(Framework::Express, None))
            }
            Expression::StaticMemberExpression(member)
                if matches!(member.property.name.as_str(), "basePath" | "route") =>
            {
                let mut binding = self.object_binding(&member.object)?;
                if let Some(prefix) = call.arguments.first().and_then(|arg| self.literal_arg(arg)) {
                    binding.prefixes.push(prefix);
                }
                Some(binding)
            }
            _ => None,
        }
    }

    fn object_binding(&self, object: &Expression<'_>) -> Option<Binding> {
        if let Expression::Identifier(id) = object {
            if let Some(binding) = self.facts.bindings.get(id.name.as_str()) {
                return Some(binding.clone());
            }
        }
        self.binding_from_expr(object)
    }
}

fn is_client_http_module(source: &str) -> bool {
    matches!(
        source,
        "axios"
            | "got"
            | "ky"
            | "supertest"
            | "superagent"
            | "undici"
            | "node-fetch"
            | "http"
            | "https"
            | "node:http"
            | "node:https"
            | "@playwright/test"
    )
}
