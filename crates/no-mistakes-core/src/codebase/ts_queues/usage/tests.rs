use super::*;

// ── Enqueue calls ────────────────────────────────────────────────────────

#[test]
fn detects_add_call_with_job_name() {
    let src = r#"
import { emailsQueue } from './queues.mts';
emailsQueue.add('sendWelcome', { userId });
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.enqueue_calls.len(), 1);
    let call = &usage.enqueue_calls[0];
    assert_eq!(call.binding, "emailsQueue");
    assert_eq!(call.job.as_deref(), Some("sendWelcome"));
    assert_eq!(call.line, 3);
}

#[test]
fn detects_add_call_dynamic_job_is_none() {
    let src = r#"
import { q } from './queues.mts';
q.add(jobType, payload);
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.enqueue_calls.len(), 1);
    assert_eq!(usage.enqueue_calls[0].job, None);
}

#[test]
fn detects_add_bulk_call() {
    let src = r#"
import { q } from './queues.mts';
q.addBulk([{ name: 'jobA', data: {} }, { name: 'jobB', data: {} }]);
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.enqueue_calls.len(), 2);
    assert_eq!(usage.enqueue_calls[0].job.as_deref(), Some("jobA"));
    assert_eq!(usage.enqueue_calls[1].job.as_deref(), Some("jobB"));
}

#[test]
fn captures_imports() {
    let src = r#"
import { emailsQueue } from './queues.mts';
import { other } from './other.mts';
"#;
    let usage = extract_queue_usage(src);
    assert!(usage
        .imports
        .contains(&("emailsQueue".to_string(), "./queues.mts".to_string())));
    assert!(usage
        .imports
        .contains(&("other".to_string(), "./other.mts".to_string())));
}

// ── Worker declarations ──────────────────────────────────────────────────

#[test]
fn detects_new_worker_with_queue_name() {
    let src = r#"
import * as processors from './processors.mts';
import { Worker } from 'glide-mq';
export const w = new Worker('emails', (job) => processors[job.name](job.data));
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.worker_declarations.len(), 1);
    let w = &usage.worker_declarations[0];
    assert_eq!(w.queue_name.as_deref(), Some("emails"));
    assert_eq!(w.processors_specifier.as_deref(), Some("./processors.mts"));
}

#[test]
fn worker_without_processors_import() {
    let src = r#"
export const w = new Worker('emails', (job) => doStuff(job));
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.worker_declarations.len(), 1);
    let w = &usage.worker_declarations[0];
    assert_eq!(w.queue_name.as_deref(), Some("emails"));
    assert_eq!(w.processors_specifier, None);
}

#[test]
fn worker_dynamic_queue_name() {
    let src = r#"
export const w = new Worker(QUEUE_NAME, handler);
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.worker_declarations.len(), 1);
    assert_eq!(usage.worker_declarations[0].queue_name, None);
}

// ── Mixed source ──────────────────────────────────────────────────────────

#[test]
fn handles_mixed_source() {
    let src = r#"
import { emailsQueue } from './queues.mts';
import * as processors from './processors.mts';
export const enqueueSendWelcome = (id: string) => {
  emailsQueue.add('sendWelcome', { id }).catch(console.error);
};
export const worker = new Worker('emails', (job) => processors[job.name](job.data));
"#;
    let usage = extract_queue_usage(src);
    assert_eq!(usage.enqueue_calls.len(), 1);
    assert_eq!(usage.enqueue_calls[0].job.as_deref(), Some("sendWelcome"));
    assert_eq!(usage.worker_declarations.len(), 1);
    assert_eq!(
        usage.worker_declarations[0].queue_name.as_deref(),
        Some("emails")
    );
}

#[test]
fn no_queue_usage_in_plain_source() {
    let src = r#"
export const foo = () => 42;
"#;
    let usage = extract_queue_usage(src);
    assert!(usage.enqueue_calls.is_empty());
    assert!(usage.worker_declarations.is_empty());
}
