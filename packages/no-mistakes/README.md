# no-mistakes

Static TS/JS codebase analysis commands.

```bash
npm install --save-dev no-mistakes
npx no-mistakes dependencies src/main.mts --json
npx no-mistakes dependents src/utils.mts --json
npx no-mistakes symbols src/utils.mts --json
```

The native binary exposes scoped subcommands only: `no-mistakes dependencies`,
`no-mistakes dependents`, and `no-mistakes symbols`.
