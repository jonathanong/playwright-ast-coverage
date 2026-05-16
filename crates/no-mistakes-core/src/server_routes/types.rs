use serde::Serialize;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Framework {
    ApiServer,
    Express,
    Hono,
    KoaPathMatch,
    KoaRouter,
    Heuristic,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRoute {
    pub file: String,
    pub line: usize,
    pub method: String,
    pub route: String,
    pub raw_path: String,
    pub framework: Framework,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    ServerRoute,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub total_routes: usize,
    pub total_files: usize,
    pub dynamic_routes: usize,
}
