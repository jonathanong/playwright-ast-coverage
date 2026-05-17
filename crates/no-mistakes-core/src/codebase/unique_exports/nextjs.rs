pub(super) fn is_framework_export(rel: &str, name: &str, is_nextjs_project: bool) -> bool {
    if !is_nextjs_project {
        return false;
    }
    let rel = rel.replace('\\', "/");
    let file_name = rel.rsplit('/').next().unwrap_or("");
    let stem = convention_stem(file_name);
    if is_app_router_path(&rel) {
        return match stem {
            "page" | "layout" => is_app_page_or_layout_export(name),
            "route" => is_app_route_export(name),
            _ => false,
        };
    }
    if is_pages_router_path(&rel) {
        return is_pages_router_export(name);
    }
    false
}

fn convention_stem(file_name: &str) -> &str {
    for extension in [".tsx", ".ts", ".jsx", ".js", ".mts", ".cts", ".mjs", ".cjs"] {
        if let Some(stem) = file_name.strip_suffix(extension) {
            return stem;
        }
    }
    file_name
}

fn is_app_router_path(rel: &str) -> bool {
    rel.starts_with("app/") || rel.contains("/app/")
}

fn is_pages_router_path(rel: &str) -> bool {
    rel.starts_with("pages/") || rel.contains("/pages/")
}

fn is_app_page_or_layout_export(name: &str) -> bool {
    matches!(
        name,
        "metadata" | "generateMetadata" | "viewport" | "generateViewport" | "generateStaticParams"
    ) || is_route_segment_config_export(name)
}

fn is_app_route_export(name: &str) -> bool {
    matches!(
        name,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    ) || is_route_segment_config_export(name)
}

fn is_route_segment_config_export(name: &str) -> bool {
    matches!(
        name,
        "dynamic"
            | "dynamicParams"
            | "revalidate"
            | "fetchCache"
            | "runtime"
            | "preferredRegion"
            | "maxDuration"
            | "experimental_ppr"
    )
}

fn is_pages_router_export(name: &str) -> bool {
    matches!(
        name,
        "getStaticProps" | "getStaticPaths" | "getServerSideProps" | "config" | "reportWebVitals"
    )
}
