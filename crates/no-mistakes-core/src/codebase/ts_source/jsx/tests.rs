use super::*;
use oxc::allocator::Allocator;
use oxc::ast::ast::JSXElementName;
use oxc::parser::Parser;
use oxc::span::SourceType;
use std::path::PathBuf;

fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> oxc::ast::ast::Program<'a> {
    Parser::new(allocator, source, SourceType::tsx())
        .parse()
        .program
}

fn fixture_source(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/ast-snippets/ts-source")
        .join(name);
    std::fs::read_to_string(path).expect("fixture source must be readable")
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
    struct First(Vec<String>);
    impl Visitor for First {
        fn visit_jsx_opening(&mut self, opening: &JSXOpeningElement) {
            if let Some(name) = jsx_identifier_name(opening) {
                self.0.push(name.to_string());
            }
        }
    }

    let allocator = Allocator::default();
    let program = parse(&allocator, "const x = <><div /><Link href=\"/a\" /></>;");
    let mut f = First(Vec::new());
    walk_program(&program, &mut f);
    assert_eq!(f.0, vec!["div", "Link"]);
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
fn default_visitor_hooks_and_non_identifier_attrs_are_exercised() {
    struct Noop;
    impl Visitor for Noop {}

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        r#"
        import x from "x";
        const value = <ns:tag ns:attr="x" dynamic={value} {...props}>{/* empty */}</ns:tag>;
        "#,
    );
    let mut noop = Noop;
    walk_program(&program, &mut noop);

    struct Grab {
        tag: Option<String>,
        dynamic: Option<(bool, Option<String>)>,
        missing: bool,
    }
    impl Visitor for Grab {
        fn visit_jsx_opening(&mut self, opening: &JSXOpeningElement) {
            self.tag = jsx_identifier_name(opening).map(str::to_string);
            self.dynamic = find_string_attr(opening, "dynamic")
                .map(|(present, value)| (present, value.map(str::to_string)));
            self.missing = find_string_attr(opening, "missing").is_none();
        }
    }
    let mut grab = Grab {
        tag: Some("unchanged".to_string()),
        dynamic: None,
        missing: false,
    };
    walk_program(&program, &mut grab);

    assert_eq!(grab.tag, None);
    assert_eq!(grab.dynamic, Some((true, None)));
    assert!(grab.missing);
}

#[test]
fn jsx_attr_helpers_cover_empty_and_non_string_values() {
    struct Grab {
        empty: Option<(bool, Option<String>)>,
        number: Option<(bool, Option<String>)>,
        element: Option<(bool, Option<String>)>,
        spread_seen: bool,
    }
    impl Visitor for Grab {
        fn visit_jsx_opening(&mut self, opening: &JSXOpeningElement) {
            self.empty = find_string_attr(opening, "empty")
                .map(|(present, value)| (present, value.map(str::to_string)));
            self.number = find_string_attr(opening, "number")
                .map(|(present, value)| (present, value.map(str::to_string)));
            self.element = find_string_attr(opening, "element")
                .map(|(present, value)| (present, value.map(str::to_string)));
            self.spread_seen |= find_string_attr(opening, "spread").is_none();
        }
    }

    let allocator = Allocator::default();
    let program = parse(
        &allocator,
        r#"const value = <Link empty={} number={1} element=<span /> {...props} />;"#,
    );
    let mut grab = Grab {
        empty: None,
        number: None,
        element: None,
        spread_seen: false,
    };
    walk_program(&program, &mut grab);
    assert_eq!(grab.empty, Some((true, None)));
    assert_eq!(grab.number, Some((true, None)));
    assert_eq!(grab.element, Some((true, None)));
    assert!(grab.spread_seen);
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

#[test]
fn walker_visits_statement_expression_and_jsx_variants_from_fixture() {
    struct Counts {
        expressions: usize,
        jsx_openings: usize,
        jsx_containers: usize,
        imports: usize,
    }
    impl Visitor for Counts {
        fn visit_import(&mut self, _import: &oxc::ast::ast::ImportDeclaration) {
            self.imports += 1;
        }

        fn visit_expression(&mut self, _expr: &Expression) {
            self.expressions += 1;
        }

        fn visit_jsx_opening(&mut self, _opening: &JSXOpeningElement) {
            self.jsx_openings += 1;
        }

        fn visit_jsx_expression_container(&mut self, _expr: &JSXExpression, _span_start: u32) {
            self.jsx_containers += 1;
        }
    }

    let source = fixture_source("jsx-walk-all.tsx");
    let allocator = Allocator::default();
    let program = parse(&allocator, &source);
    let mut counts = Counts {
        expressions: 0,
        jsx_openings: 0,
        jsx_containers: 0,
        imports: 0,
    };
    walk_program(&program, &mut counts);

    assert_eq!(counts.imports, 1);
    assert!(counts.expressions > 70, "{:?}", counts.expressions);
    assert!(counts.jsx_openings >= 8, "{:?}", counts.jsx_openings);
    assert!(counts.jsx_containers >= 7, "{:?}", counts.jsx_containers);
}

#[test]
fn program_contains_jsx_walks_fixture_statement_shapes() {
    let source = fixture_source("jsx-walk-all.tsx");
    let allocator = Allocator::default();
    let program = parse(&allocator, &source);
    assert!(program_contains_jsx(&program));
}

#[test]
fn program_contains_jsx_walks_non_jsx_fixture_statement_shapes() {
    let source = fixture_source("no-jsx-walk-all.ts");
    let allocator = Allocator::default();
    let program = parse(&allocator, &source);
    assert!(!program_contains_jsx(&program));
}

#[test]
fn optional_walk_helpers_visit_present_nodes() {
    struct Count(usize);
    impl Visitor for Count {
        fn visit_expression(&mut self, _expr: &Expression) {
            self.0 += 1;
        }
    }

    let source = fixture_source("no-jsx-walk-all.ts");
    let allocator = Allocator::default();
    let program = parse(&allocator, &source);
    let mut count = Count(0);

    let if_stmt = program
        .body
        .iter()
        .find_map(|stmt| match stmt {
            Statement::IfStatement(if_stmt) => Some(if_stmt),
            _ => None,
        })
        .expect("fixture must contain an if statement");
    walk_optional_statement(if_stmt.alternate.as_ref(), &mut count);

    let export = program
        .body
        .iter()
        .find_map(|stmt| match stmt {
            Statement::ExportNamedDeclaration(export) => Some(export),
            _ => None,
        })
        .expect("fixture must contain a named export");
    walk_optional_declaration(export.declaration.as_ref(), &mut count);

    let var_decl = program
        .body
        .iter()
        .find_map(|stmt| match stmt {
            Statement::VariableDeclaration(var_decl) => Some(var_decl),
            _ => None,
        })
        .expect("fixture must contain a variable declaration");
    walk_optional_expression(var_decl.declarations[0].init.as_ref(), &mut count);

    assert!(count.0 > 0);
}

#[test]
fn walker_visits_default_exports_and_edge_expression_shapes() {
    struct Counts {
        expressions: usize,
        openings: usize,
        containers: usize,
    }
    impl Visitor for Counts {
        fn visit_expression(&mut self, _expr: &Expression) {
            self.expressions += 1;
        }

        fn visit_jsx_opening(&mut self, _opening: &JSXOpeningElement) {
            self.openings += 1;
        }

        fn visit_jsx_expression_container(&mut self, _expr: &JSXExpression, _span_start: u32) {
            self.containers += 1;
        }
    }

    for source in [
        r#"
        export default function DefaultFn() {
          return <div>{value}</div>;
        }
        "#,
        r#"
        export default class DefaultClass {
          render() {
            return <section>{items?.[key]}</section>;
          }
        }
        "#,
        r#"
        export default (ready ? <A attr={value as string} /> : <B {...props}>{...children}</B>);
        "#,
        r#"
        const value = (
          (target["key"] = call(...args, ...more)),
          target?.[key],
          tag`value-${expr}`,
          <ns:Tag attr={}>{value}</ns:Tag>
        );
        "#,
    ] {
        let allocator = Allocator::default();
        let program = parse(&allocator, source);
        let mut counts = Counts {
            expressions: 0,
            openings: 0,
            containers: 0,
        };
        walk_program(&program, &mut counts);
        assert!(counts.expressions > 0);
    }
}

#[test]
fn walker_directly_exercises_sparse_statement_and_expression_branches() {
    struct Count(usize);
    impl Visitor for Count {
        fn visit_expression(&mut self, _expr: &Expression) {
            self.0 += 1;
        }
    }

    let source = r#"
declare function ambient(): void;
declare class Ambient { render(): void; }

for (;;) {
  break;
}

try {
  value;
} catch {
  caught;
}

try {
  value;
} finally {
  cleaned;
}

switch (kind) {
  default:
    fallback;
}

function* gen() {
  yield;
  yield value;
}

class Mixed {
  field = ignored;
  method() {
    this.value;
  }
}

export function declared() {
  return <Declared />;
}

export class DeclaredClass {
  field = ignored;
  method() {
    return <DeclaredClassView />;
  }
}

	export default class DefaultWithField {
	  field = ignored;
	  method() {
	    return <DefaultClassView />;
	  }
	}

	export default function DefaultFunction() {
	  <DefaultFunctionView />;
	}

	export function NamedFunction() {
	  <NamedFunctionView />;
	}

	export class NamedClass {
	  method() {
	    <NamedClassView />;
	  }
	}

	export enum ExportedEnum {
	  A,
	}

	const chainCall = target?.method?.(arg);
	const chainMember = target?.[key];
	const assignment = (target[key] = value);
	const update = target[key]++;
	const optionalStatic = target?.prop;
	const tagged = tag`literal`;
	const array = [first, , ...rest];
	const object = { a: first, ...rest };
	const typed = (await (value as string)!) satisfies unknown;
	const fnExpr = function () {
	  nested;
	};
	"#;
    let allocator = Allocator::default();
    let program = parse(&allocator, source);
    let mut count = Count(0);
    walk_program(&program, &mut count);
    assert!(count.0 > 20, "visited {} expressions", count.0);
}

#[test]
fn walker_visits_remaining_expression_shapes() {
    struct Count(usize);
    impl Visitor for Count {
        fn visit_expression(&mut self, _expr: &Expression) {
            self.0 += 1;
        }
    }
    let allocator = Allocator::default();
    let program = Parser::new(
        &allocator,
        "export const value = (!flag, <string>raw, fn(1, ...args), new Thing(1, ...args), maybe?.method?.(1, ...args), maybe?.plain, function () { inner; });",
        SourceType::ts(),
    ).parse().program;
    let mut count = Count(0);
    walk_program(&program, &mut count);
    assert!(count.0 > 20, "visited {} expressions", count.0);
}
