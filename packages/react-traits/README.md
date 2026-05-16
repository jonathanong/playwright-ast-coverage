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

# Assert no component calls fetch directly
react-traits check 'app/components/**/*.tsx' --assert-no-fetch

# JSON output
react-traits analyze 'app/components/**/*.tsx' --json
```

## Options

- `--root <path>` — repo root (default: `.`)
- `--config <path>` — config file path
- `--json` — emit JSON instead of text

## Configuration

Place a `.no-mistakes.yaml` or `.react-traits.yaml` at your repo root:

```yaml
frontendRoot: app
reactTraits:
  assertNoFetch: true
```

## License

MIT
