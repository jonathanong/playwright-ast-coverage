use super::*;
use oxc::allocator::Allocator;
use oxc::ast::ast::JSXElementName;
use oxc::parser::Parser;
use oxc::span::SourceType;

fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> oxc::ast::ast::Program<'a> {
    Parser::new(allocator, source, SourceType::tsx())
        .parse()
        .program
}

#[test]
fn detects_jsx_presence() {
    let allocator = Allocator::default();
    assert!(program_contains_jsx(&parse(
        &allocator,
        "export const X = () => <div>hi</div>;",
    )));
}

#[test]
fn detects_no_jsx_in_pure_ts() {
    let allocator = Allocator::default();
    assert!(!program_contains_jsx(&parse(
        &allocator,
        "export const add = (a: number, b: number) => a + b;",
    )));
}

#[test]
fn detects_fragment_as_jsx() {
    let allocator = Allocator::default();
    assert!(program_contains_jsx(&parse(
        &allocator,
        "export const X = () => <>hi</>;",
    )));
}

#[test]
fn walk_program_visits_nested_jsx_openings() {
    struct Collect(Vec<String>);
    impl Visitor for Collect {
        fn visit_jsx_opening(&mut self, opening: &JSXOpeningElement) {
            if let JSXElementName::Identifier(id) = &opening.name {
                self.0.push(id.name.to_string());
            }
        }
    }

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        r#"
        export const X = () => (
            <div>
                <span>inner</span>
                {true && <img src="/x.png" />}
            </div>
        );
        "#,
    );
    let mut c = Collect(Vec::new());
    walk_program(&program, &mut c);
    assert_eq!(c.0, vec!["div", "span", "img"]);
}

#[test]
fn jsx_identifier_name_returns_tag() {
    struct First(Option<String>);
    impl Visitor for First {
        fn visit_jsx_opening(&mut self, opening: &JSXOpeningElement) {
            if self.0.is_none() {
                self.0 = jsx_identifier_name(opening).map(|s| s.to_string());
            }
        }
    }

    let allocator = Allocator::default();
    let program = parse(&allocator, "const x = <Link href=\"/a\" />;");
    let mut f = First(None);
    walk_program(&program, &mut f);
    assert_eq!(f.0.as_deref(), Some("Link"));
}

#[test]
fn find_string_attr_reads_string_literal_and_expression_container() {
    struct Grab {
        target: Option<String>,
        rel: Option<String>,
        dynamic: Option<bool>,
    }
    impl Visitor for Grab {
        fn visit_jsx_opening(&mut self, opening: &JSXOpeningElement) {
            if let Some((_, Some(v))) = find_string_attr(opening, "target") {
                self.target = Some(v.to_string());
            }
            if let Some((_, Some(v))) = find_string_attr(opening, "rel") {
                self.rel = Some(v.to_string());
            }
            if let Some((present, value)) = find_string_attr(opening, "dynamic") {
                // boolean shorthand -> (true, None)
                self.dynamic = Some(present && value.is_none());
            }
        }
    }

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        r#"const x = <a target="_blank" rel={"nofollow"} dynamic />;"#,
    );
    let mut g = Grab {
        target: None,
        rel: None,
        dynamic: None,
    };
    walk_program(&program, &mut g);
    assert_eq!(g.target.as_deref(), Some("_blank"));
    assert_eq!(g.rel.as_deref(), Some("nofollow"));
    assert_eq!(g.dynamic, Some(true));
}

#[test]
fn visit_expression_hits_assignments_inside_jsx_handlers() {
    struct CountAssigns(usize);
    impl Visitor for CountAssigns {
        fn visit_expression(&mut self, expr: &Expression) {
            if matches!(expr, Expression::AssignmentExpression(_)) {
                self.0 += 1;
            }
        }
    }

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        r#"
        export const X = () => (
            <button onClick={() => { window.location.href = "/x"; }}>click</button>
        );
        "#,
    );
    let mut c = CountAssigns(0);
    walk_program(&program, &mut c);
    assert_eq!(c.0, 1);
}

#[test]
fn visits_import_declarations() {
    struct Imports(Vec<String>);
    impl Visitor for Imports {
        fn visit_import(&mut self, import: &oxc::ast::ast::ImportDeclaration) {
            self.0.push(import.source.value.to_string());
        }
    }

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        "import Link from \"next/link\"; import { a } from \"@/lib\";",
    );
    let mut i = Imports(Vec::new());
    walk_program(&program, &mut i);
    assert_eq!(i.0, vec!["next/link", "@/lib"]);
}

#[test]
fn visit_expression_hits_assignment_inside_spread_attr() {
    struct CountAssigns(usize);
    impl Visitor for CountAssigns {
        fn visit_expression(&mut self, expr: &Expression) {
            if matches!(expr, Expression::AssignmentExpression(_)) {
                self.0 += 1;
            }
        }
    }

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        r#"
        export const X = () => (
            <Comp {...{ onClick: () => { window.location.href = "/x"; } }} />
        );
        "#,
    );
    let mut c = CountAssigns(0);
    walk_program(&program, &mut c);
    assert_eq!(c.0, 1);
}
