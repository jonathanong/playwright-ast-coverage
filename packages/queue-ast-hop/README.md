# queue-ast-hop

Static queue dependency graph for BullMQ and glide-mq.

```sh
npm install --save-dev queue-ast-hop
npx queue-ast-hop edges --json
npx queue-ast-hop related backend/jobs/email.ts
npx queue-ast-hop check
```

`queue-ast-hop` reports producer-to-worker hops through virtual `queueFile#job`
nodes so changed-file impact analysis can cross asynchronous job queues.

See the [documentation index](../../docs/README.md) and
[CLI reference](../../docs/cli-reference.md).
