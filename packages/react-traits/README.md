# react-traits

Statically scan React/Next.js component files and report each component's traits (state, props, memoization, fetch, environment, etc.) plus the components it renders.

## Installation

```sh
npm install react-traits
```

## Usage

```sh
# List components and their traits
react-traits analyze 'app/components/**/*.tsx'

# Assert no component (or any component in its rendered subtree) calls fetch
react-traits check 'app/components/**/*.tsx' --assert-no-fetch

# JSON output
react-traits analyze 'app/components/**/*.tsx' --json
```

## Options

Global options (available on all subcommands):

- `--root <path>` — repo root (default: `.`)
- `--config <path>` — config file path
- `--json` — emit JSON instead of text

### `check` subcommand options

- `--assert-no-fetch` — exit non-zero if any component or its rendered subtree calls fetch

## Configuration

Place a `.no-mistakes.yaml` or `.react-traits.yaml` at your repo root:

```yaml
frontendRoot: app
reactTraits:
  assertNoFetch: true
```

## License

MIT

See the [documentation index](../../docs/README.md), [CLI reference](../../docs/cli-reference.md),
and [AST analysis behavior](../../docs/ast-analysis.md) for full behavior notes.
