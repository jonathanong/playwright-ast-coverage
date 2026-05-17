use super::common::{file_paths, fixture, run_json, via_kinds};

#[test]
fn cross_boundary_workspace_and_symbol_contracts() {
    let root = fixture("cross-boundary-monorepo");
    let backend_tsconfig = root.join("apps/backend/tsconfig.json");
    let web_tsconfig = root.join("apps/web/tsconfig.json");
    let root_tsconfig = root.join("tsconfig.json");
    let backend_tsconfig = backend_tsconfig.to_string_lossy();
    let web_tsconfig = web_tsconfig.to_string_lossy();
    let root_tsconfig = root_tsconfig.to_string_lossy();

    let workspace_deps = run_json(
        &root,
        &[
            "dependencies",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "--relationship",
            "workspace",
            "apps/backend/api/handler.mts",
        ],
    );
    assert!(file_paths(&workspace_deps).contains(&"packages/core/src/index.mts".to_string()));

    let workspace_reverse = run_json(
        &root,
        &[
            "dependents",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "--relationship",
            "workspace",
            "packages/core/src/index.mts",
        ],
    );
    assert!(file_paths(&workspace_reverse).contains(&"apps/backend/api/handler.mts".to_string()));

    let subpath = run_json(
        &root,
        &[
            "dependents",
            "--tsconfig",
            web_tsconfig.as_ref(),
            "--relationship",
            "workspace",
            "packages/core/src/types.mts",
        ],
    );
    assert!(file_paths(&subpath).contains(&"apps/web/pages/subpath.tsx".to_string()));

    let alias_deps = run_json(
        &root,
        &[
            "dependencies",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "--relationship",
            "import",
            "apps/backend/api/handler.mts",
        ],
    );
    assert!(file_paths(&alias_deps).contains(&"apps/backend/services/topics/get.mts".to_string()));

    let alias_reverse = run_json(
        &root,
        &[
            "dependents",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "--relationship",
            "import",
            "apps/backend/services/topics/get.mts",
        ],
    );
    assert!(file_paths(&alias_reverse).contains(&"apps/backend/api/handler.mts".to_string()));

    let web_alias_deps = run_json(
        &root,
        &[
            "dependencies",
            "--tsconfig",
            web_tsconfig.as_ref(),
            "--relationship",
            "import",
            "apps/web/pages/index.tsx",
        ],
    );
    assert!(file_paths(&web_alias_deps).contains(&"packages/core/src/types.mts".to_string()));

    let web_alias_reverse = run_json(
        &root,
        &[
            "dependents",
            "--tsconfig",
            web_tsconfig.as_ref(),
            "--relationship",
            "import",
            "packages/core/src/types.mts",
        ],
    );
    assert!(file_paths(&web_alias_reverse).contains(&"apps/web/pages/index.tsx".to_string()));

    let full_deps = run_json(
        &root,
        &[
            "dependencies",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "apps/backend/api/handler.mts",
        ],
    );
    let types_path = "apps/backend/services/topics/types.mts";
    assert!(file_paths(&full_deps).contains(&types_path.to_string()));
    assert!(via_kinds(&full_deps, types_path).contains(&"type-import".to_string()));
    assert!(!via_kinds(&full_deps, types_path).contains(&"import".to_string()));

    let symbols = run_json(&root, &["symbols", "packages/core/src/index.mts"]);
    let exports = symbols["files"][0]["exports"].as_array().unwrap();
    let internal_helper = exports
        .iter()
        .find(|export| export["name"].as_str() == Some("internalHelper"))
        .unwrap();
    assert_eq!(internal_helper["kind"], "re-export");

    let symbol_dependents = run_json(
        &root,
        &[
            "dependents",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "packages/core/src/internal.mts#internalHelper",
        ],
    );
    assert!(file_paths(&symbol_dependents).contains(&"apps/backend/api/handler.mts".to_string()));

    let serial = run_json(
        &root,
        &[
            "-j",
            "1",
            "dependents",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "packages/core/src/internal.mts#internalHelper",
        ],
    );
    let parallel = run_json(
        &root,
        &[
            "-j",
            "8",
            "dependents",
            "--tsconfig",
            backend_tsconfig.as_ref(),
            "packages/core/src/internal.mts#internalHelper",
        ],
    );
    assert_eq!(parallel, serial);

    let extends = run_json(
        &root,
        &[
            "dependencies",
            "--tsconfig",
            root_tsconfig.as_ref(),
            "apps/backend/api/core-client.mts",
        ],
    );
    assert!(file_paths(&extends)
        .iter()
        .any(|path| path.starts_with("packages/core/")));

    let base_url = run_json(
        &root,
        &[
            "dependencies",
            "--tsconfig",
            root_tsconfig.as_ref(),
            "--relationship",
            "import",
            "apps/backend/api/baseurl-client.mts",
        ],
    );
    assert!(file_paths(&base_url).contains(&"packages/core/src/internal.mts".to_string()));
}
