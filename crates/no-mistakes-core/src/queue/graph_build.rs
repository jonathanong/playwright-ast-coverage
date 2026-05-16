use crate::queue::extract::FileFacts;
use crate::queue::graph_model::{
    dedup_sorted, diagnostics, node_name, InternalProducer, InternalWorker, ProjectReport,
};
use crate::queue::source::relative_string;
use crate::queue::types::{Edge, EdgeKind, JobKey, QueueJobNode, QueueKey};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn build_report(
    root: &Path,
    producers: Vec<InternalProducer>,
    workers: Vec<InternalWorker>,
    facts: &HashMap<PathBuf, FileFacts>,
) -> ProjectReport {
    let producer_index = index_producers(&producers);
    let worker_index = index_workers(&workers);
    let wildcards = wildcard_queues(&workers);
    let mut jobs = Vec::new();
    let mut edges = Vec::new();
    let mut check = Vec::new();
    for (job, producers_for_job) in &producer_index {
        let workers_for_job = worker_index.get(job).cloned().unwrap_or_default();
        if workers_for_job.is_empty() && !wildcards.contains(&queue_key(job)) {
            check.extend(
                producers_for_job
                    .iter()
                    .map(|producer| producer.unmatched(root)),
            );
            continue;
        }
        add_matched_job(
            root,
            job,
            producers_for_job,
            &workers_for_job,
            &mut jobs,
            &mut edges,
        );
    }
    for worker in &workers {
        for (job, _) in worker.job_keys() {
            if !producer_index.contains_key(&job) {
                check.push(worker.unmatched(root, &job));
            }
        }
    }
    ProjectReport {
        producers: producers.iter().map(|p| p.public(root)).collect(),
        workers: workers.iter().map(|w| w.public(root)).collect(),
        jobs: dedup_sorted(jobs),
        edges: dedup_sorted(edges),
        diagnostics: diagnostics(root, facts, &producers, &workers),
        check: dedup_sorted(check),
    }
}

fn index_producers(producers: &[InternalProducer]) -> HashMap<JobKey, Vec<&InternalProducer>> {
    producers
        .iter()
        .filter_map(|p| p.job_key())
        .fold(HashMap::new(), |mut map, producer| {
            map.entry(producer.0)
                .or_insert_with(Vec::new)
                .push(producer.1);
            map
        })
}

fn index_workers(workers: &[InternalWorker]) -> HashMap<JobKey, Vec<&InternalWorker>> {
    workers
        .iter()
        .flat_map(InternalWorker::job_keys)
        .fold(HashMap::new(), |mut map, worker| {
            map.entry(worker.0).or_insert_with(Vec::new).push(worker.1);
            map
        })
}

fn wildcard_queues(workers: &[InternalWorker]) -> HashSet<QueueKey> {
    workers
        .iter()
        .filter(|w| w.site.wildcard)
        .filter_map(|w| w.queue.clone())
        .collect()
}

fn add_matched_job(
    root: &Path,
    job: &JobKey,
    producers: &[&InternalProducer],
    workers: &[&InternalWorker],
    jobs: &mut Vec<QueueJobNode>,
    edges: &mut Vec<Edge>,
) {
    let node = node_name(root, job);
    jobs.push(QueueJobNode {
        queue_file: relative_string(root, &job.queue_file),
        queue_name: job.queue_name.clone(),
        job: job.job.clone(),
    });
    edges.extend(producers.iter().map(|producer| Edge {
        from: relative_string(root, &producer.site.file),
        to: node.clone(),
        kind: EdgeKind::QueueEnqueue,
    }));
    edges.extend(workers.iter().map(|worker| {
        Edge {
            from: node.clone(),
            to: relative_string(
                root,
                worker
                    .site
                    .processor_file
                    .as_ref()
                    .unwrap_or(&worker.site.file),
            ),
            kind: EdgeKind::QueueWorker,
        }
    }));
}

fn queue_key(job: &JobKey) -> QueueKey {
    QueueKey {
        queue_file: job.queue_file.clone(),
        queue_name: job.queue_name.clone(),
    }
}
