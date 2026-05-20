use super::{collect_from_expr, HttpCall};
use oxc::ast::ast::AssignmentTarget;

pub(super) fn collect_from_assignment_target(
    target: &AssignmentTarget,
    source: &str,
    prefixes: &[&str],
    out: &mut Vec<HttpCall>,
) {
    if let AssignmentTarget::ComputedMemberExpression(member) = target {
        collect_from_expr(&member.object, source, prefixes, out);
        collect_from_expr(&member.expression, source, prefixes, out);
    }
}
