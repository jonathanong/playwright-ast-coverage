use crate::queue::extract::FileFacts;
use crate::queue::source::relative_string;
use crate::queue::types::{
    Diagnostic, Edge, JobKey, QueueJobNode, QueueKey, QueueProducer, QueueWorker, Severity,
};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckFinding {
    pub kind: String,
    pub file: String,
    pub line: usize,
    pub queue_file: Option<String>,
    pub queue_name: Option<String>,
    pub job: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReport {
    pub producers: Vec<QueueProducer>,
    pub workers: Vec<QueueWorker>,
    pub jobs: Vec<QueueJobNode>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<Diagnostic>,
    pub check: Vec<CheckFinding>,
}

#[derive(Debug, Clone)]
pub(super) struct InternalProducer {
    pub site: crate::queue::extract::ProducerSite,
    pub queue: Option<QueueKey>,
}

#[derive(Debug, Clone)]
pub(super) struct InternalWorker {
    pub site: crate::queue::extract::WorkerSite,
    pub queue: Option<QueueKey>,
}

impl InternalProducer {
    pub(super) fn job_key(&self) -> Option<(JobKey, &Self)> {
        let queue = self.queue.as_ref()?;
        Some((
            JobKey {
                queue_file: queue.queue_file.clone(),
                queue_name: queue.queue_name.clone(),
                job: self.site.job.clone()?,
            },
            self,
        ))
    }

    pub(super) fn public(&self, root: &Path) -> QueueProducer {
        QueueProducer {
            file: relative_string(root, &self.site.file),
            line: self.site.line,
            queue_file: self
                .queue
                .as_ref()
                .map(|q| relative_string(root, &q.queue_file)),
            queue_name: self.queue.as_ref().map(|q| q.queue_name.clone()),
            job: self.site.job.clone(),
            raw_job: self.site.raw_job.clone(),
            library: None,
        }
    }

    pub(super) fn unmatched(&self, root: &Path) -> CheckFinding {
        CheckFinding {
            kind: "unmatched-producer".to_string(),
            file: relative_string(root, &self.site.file),
            line: self.site.line,
            queue_file: self
                .queue
                .as_ref()
                .map(|q| relative_string(root, &q.queue_file)),
            queue_name: self.queue.as_ref().map(|q| q.queue_name.clone()),
            job: self.site.job.clone(),
            message: "static queue producer has no matching worker".to_string(),
        }
    }
}

impl InternalWorker {
    pub(super) fn job_keys(&self) -> Vec<(JobKey, &Self)> {
        let Some(queue) = &self.queue else {
            return Vec::new();
        };
        self.site
            .jobs
            .iter()
            .map(|job| {
                (
                    JobKey {
                        queue_file: queue.queue_file.clone(),
                        queue_name: queue.queue_name.clone(),
                        job: job.clone(),
                    },
                    self,
                )
            })
            .collect()
    }

    pub(super) fn public(&self, root: &Path) -> QueueWorker {
        QueueWorker {
            file: relative_string(root, &self.site.file),
            line: self.site.line,
            processor_file: self
                .site
                .processor_file
                .as_ref()
                .map(|p| relative_string(root, p)),
            queue_file: self
                .queue
                .as_ref()
                .map(|q| relative_string(root, &q.queue_file)),
            queue_name: self.queue.as_ref().map(|q| q.queue_name.clone()),
            jobs: self.site.jobs.clone(),
            wildcard: self.site.wildcard,
            library: None,
        }
    }

    pub(super) fn unmatched(&self, root: &Path, job: &JobKey) -> CheckFinding {
        CheckFinding {
            kind: "unmatched-worker".to_string(),
            file: relative_string(root, &self.site.file),
            line: self.site.line,
            queue_file: Some(relative_string(root, &job.queue_file)),
            queue_name: Some(job.queue_name.clone()),
            job: Some(job.job.clone()),
            message: "static queue worker has no matching producer".to_string(),
        }
    }
}

pub(super) fn diagnostics(
    root: &Path,
    facts: &HashMap<PathBuf, FileFacts>,
    producers: &[InternalProducer],
    workers: &[InternalWorker],
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (path, facts) in facts {
        for (line, message) in &facts.diagnostics {
            out.push(Diagnostic {
                severity: Severity::Warning,
                file: relative_string(root, path),
                line: *line,
                message: message.clone(),
            });
        }
    }
    out.extend(
        producers
            .iter()
            .filter_map(|p| unresolved_producer(root, p)),
    );
    out.extend(workers.iter().filter_map(|w| unresolved_worker(root, w)));
    dedup_sorted(out)
}

pub(super) fn node_name(root: &Path, job: &JobKey) -> String {
    format!("{}#{}", relative_string(root, &job.queue_file), job.job)
}

pub(super) fn build_filter(filters: &[String]) -> anyhow::Result<Option<GlobSet>> {
    if filters.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for filter in filters {
        builder.add(GlobBuilder::new(filter).literal_separator(false).build()?);
    }
    Ok(Some(builder.build()?))
}

pub(super) fn dedup_sorted<T: Ord>(mut values: Vec<T>) -> Vec<T> {
    values.sort();
    values.dedup();
    values
}

fn unresolved_producer(root: &Path, producer: &InternalProducer) -> Option<Diagnostic> {
    (producer.queue.is_none() || producer.site.job.is_none()).then(|| Diagnostic {
        severity: Severity::Warning,
        file: relative_string(root, &producer.site.file),
        line: producer.site.line,
        message: "dynamic or unresolved queue producer".to_string(),
    })
}

fn unresolved_worker(root: &Path, worker: &InternalWorker) -> Option<Diagnostic> {
    worker.queue.is_none().then(|| Diagnostic {
        severity: Severity::Warning,
        file: relative_string(root, &worker.site.file),
        line: worker.site.line,
        message: "dynamic or unresolved queue worker".to_string(),
    })
}
