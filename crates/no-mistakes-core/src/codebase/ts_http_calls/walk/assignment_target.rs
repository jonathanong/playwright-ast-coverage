use super::{collect_from_expr, HttpCall};
use oxc::ast::ast::AssignmentTarget;

pub(super) fn collect_from_assignment_target(
    target: &AssignmentTarget,
    source: &str,
    prefixes: &[&str],
    out: &mut Vec<HttpCall>,
) {
    match target {
        AssignmentTarget::ComputedMemberExpression(member) => {
            collect_from_expr(&member.object, source, prefixes, out);
            collect_from_expr(&member.expression, source, prefixes, out);
        }
        AssignmentTarget::StaticMemberExpression(member) => {
            collect_from_expr(&member.object, source, prefixes, out);
        }
        AssignmentTarget::PrivateFieldExpression(member) => {
            collect_from_expr(&member.object, source, prefixes, out);
        }
        AssignmentTarget::TSAsExpression(expr) => {
            collect_from_expr(&expr.expression, source, prefixes, out);
        }
        AssignmentTarget::TSSatisfiesExpression(expr) => {
            collect_from_expr(&expr.expression, source, prefixes, out);
        }
        AssignmentTarget::TSNonNullExpression(expr) => {
            collect_from_expr(&expr.expression, source, prefixes, out);
        }
        AssignmentTarget::TSTypeAssertion(expr) => {
            collect_from_expr(&expr.expression, source, prefixes, out);
        }
        _ => {}
    }
}
