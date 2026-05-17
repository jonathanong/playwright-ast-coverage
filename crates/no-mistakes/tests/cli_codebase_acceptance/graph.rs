use super::common::{
    file_paths, fixture, has_path_with_via, has_queue_job_with_via, run_json, via_kinds,
};

#[test]
fn import_forms_report_expected_edge_kinds() {
    let root = fixture("import-forms");
    let cases = [
        ("static.mts", "import"),
        ("type-only.mts", "type-import"),
        ("inline-type.mts", "type-import"),
        ("import-type.mts", "type-import"),
        ("dynamic.mts", "dynamic-import"),
        ("require.js", "require"),
        ("reexport.mts", "import"),
    ];

    for (source, expected_kind) in cases {
        let value = run_json(&root, &["dependencies", "--relationship", "import", source]);
        assert_eq!(file_paths(&value), vec!["target.mts"]);
        assert_eq!(via_kinds(&value, "target.mts"), vec![expected_kind]);
    }

    let dependents = run_json(
        &root,
        &["dependents", "--relationship", "import", "target.mts"],
    );
    let mut paths = file_paths(&dependents);
    paths.sort();
    assert_eq!(
        paths,
        vec![
            "dynamic.mts",
            "import-type.mts",
            "inline-type.mts",
            "reexport.mts",
            "require.js",
            "static.mts",
            "type-only.mts",
        ]
    );
    assert_eq!(
        via_kinds(&dependents, "dynamic.mts"),
        vec!["dynamic-import"]
    );
    assert_eq!(via_kinds(&dependents, "require.js"), vec!["require"]);
    assert_eq!(
        via_kinds(&dependents, "inline-type.mts"),
        vec!["type-import"]
    );
}

#[test]
fn graph_edge_kind_acceptance() {
    let root = fixture("codebase-intel");

    let vitest = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "test",
            "--depth",
            "1",
            "packages/api/src/index.mts",
        ],
    );
    assert!(has_path_with_via(
        &vitest,
        "packages/api/src/index.test.mts",
        "test"
    ));

    let md = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "md",
            "--depth",
            "1",
            "README.md",
        ],
    );
    assert!(has_path_with_via(&md, "packages/api/src/index.mts", "md"));

    let process = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "process",
            "--depth",
            "1",
            "packages/api/src/spawn-runner.mts",
        ],
    );
    assert!(has_path_with_via(
        &process,
        "packages/api/src/spawn-target.mts",
        "process"
    ));

    let route = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "route",
            "--depth",
            "1",
            "packages/web/src/api-client.tsx",
        ],
    );
    assert!(has_path_with_via(
        &route,
        "packages/api/src/index.mts",
        "route"
    ));

    let route_reverse = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "route",
            "--depth",
            "1",
            "packages/api/src/index.mts",
        ],
    );
    assert!(has_path_with_via(
        &route_reverse,
        "packages/web/src/api-client.tsx",
        "route"
    ));

    let http = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "http",
            "--depth",
            "1",
            "packages/web/src/api-client.tsx",
        ],
    );
    assert!(has_path_with_via(
        &http,
        "packages/api/src/index.mts",
        "http"
    ));

    let http_reverse = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "http",
            "--depth",
            "1",
            "packages/api/src/index.mts",
        ],
    );
    assert!(has_path_with_via(
        &http_reverse,
        "packages/web/src/api-client.tsx",
        "http"
    ));

    let playwright = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "test",
            "--depth",
            "1",
            "tests/e2e/users.spec.ts",
        ],
    );
    assert!(has_path_with_via(
        &playwright,
        "packages/web/app/users/[id]/page.tsx",
        "route-test"
    ));

    let queue_enqueue = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "queue",
            "--depth",
            "1",
            "packages/api/src/send-email.mts",
        ],
    );
    assert!(has_queue_job_with_via(
        &queue_enqueue,
        "packages/api/src/emails.mts",
        "sendWelcomeEmail",
        "queue-enqueue"
    ));

    let queue_worker = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "queue",
            "--depth",
            "1",
            "packages/api/src/processors.mts",
        ],
    );
    assert!(has_queue_job_with_via(
        &queue_worker,
        "packages/api/src/emails.mts",
        "sendWelcomeEmail",
        "queue-worker"
    ));

    let ci = run_json(
        &root,
        &[
            "dependencies",
            "--relationship",
            "ci",
            "--depth",
            "1",
            ".github/workflows/ci.yml",
        ],
    );
    assert!(has_path_with_via(&ci, "src/bin/guardrails.rs", "ci"));
    assert!(has_path_with_via(&ci, "src/bin/pg_schema.rs", "ci"));

    let ci_reverse = run_json(
        &root,
        &[
            "dependents",
            "--relationship",
            "ci",
            "--depth",
            "1",
            "src/bin/guardrails.rs",
        ],
    );
    assert!(has_path_with_via(
        &ci_reverse,
        ".github/workflows/ci.yml",
        "ci"
    ));
}
