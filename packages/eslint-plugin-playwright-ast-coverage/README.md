# eslint-plugin-playwright-ast-coverage

ESLint and Oxlint rules for keeping Playwright test IDs static, defaulted, and
consistent with `playwright-ast-coverage`.

```sh
npm install --save-dev eslint-plugin-playwright-ast-coverage
```

The plugin exports `configs.recommended` and `configs.strict` for ESLint flat
config, and the same rules can be loaded by Oxlint through `jsPlugins`.

See the full setup and rule reference in
[docs/eslint-plugin.md](../../docs/eslint-plugin.md).
