use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueJobNode {
    pub queue_file: String,
    pub queue_name: String,
    pub job: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueProducer {
    pub file: String,
    pub line: usize,
    pub queue_file: Option<String>,
    pub queue_name: Option<String>,
    pub job: Option<String>,
    pub raw_job: Option<String>,
    pub library: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueWorker {
    pub file: String,
    pub line: usize,
    pub processor_file: Option<String>,
    pub queue_file: Option<String>,
    pub queue_name: Option<String>,
    pub jobs: Vec<String>,
    pub wildcard: bool,
    pub library: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    QueueEnqueue,
    QueueWorker,
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeKind::QueueEnqueue => f.write_str("queue-enqueue"),
            EdgeKind::QueueWorker => f.write_str("queue-worker"),
        }
    }
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
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct QueueKey {
    pub queue_file: PathBuf,
    pub queue_name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct JobKey {
    pub queue_file: PathBuf,
    pub queue_name: String,
    pub job: String,
}
