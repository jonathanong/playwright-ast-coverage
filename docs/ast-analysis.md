# AST Analysis

`playwright-ast-coverage` uses the Oxc parser to inspect JavaScript,
TypeScript, JSX, and TSX source. It does not run tests or execute project code.
Only static forms described here are detected.

## Playwright Config

The tool parses root-level `playwright*.config.*` files discovered under
`--root`, paths from analyzer config `playwrightConfig`, or repeated
`--playwright-config` options.

Supported config shapes:

```ts
export default { testDir: './tests' }
export default defineConfig({ testDir: './tests' })

const config = { testDir: './tests' }
export default config

module.exports = { testDir: './tests' }
module.exports = defineConfig({ testDir: './tests' })
```

It can follow top-level object bindings used as the exported config, the
`defineConfig(...)` argument, or the `use` object. Cyclic bindings are ignored.

Supported literal fields:

- `name`
- `testDir`
- `testMatch`
- `testIgnore`
- `baseURL` and `use.baseURL`
- `testIdAttribute` and `use.testIdAttribute`
- `projects`, when entries are object literals

`testDir`, `baseURL`, `testIdAttribute`, and `name` must be string literals or
expression-free template literals. `testMatch` and `testIgnore` may be a string
literal or an array of string literals. Regular-expression `testMatch` and
`testIgnore` patterns are not supported.

## Route Files

Routes are collected from Next.js App Router files under `frontendRoot`:

- `page.ts`
- `page.tsx`
- `page.js`
- `page.jsx`

Route groups like `(admin)` and parallel route segments like `@modal` are
ignored when building route patterns. Dynamic segments map as follows:

| App Router segment | Route pattern segment |
| --- | --- |
| `[id]` | `:id` |
| `[...rest]` | `*` |
| `[[...rest]]` | `**` |

## Test URL Detection

Tests cover routes when a detected URL normalizes to a local path and that path
matches a route pattern.

Detected AST forms:

```ts
await page.goto('/users/42');
await page.goto("http://localhost:3000/users/42");
await page.goto(`/users/${id}`);
await page.click('a[href="/settings"]');
await page.click(`a[href='/settings']`);
await expect(page).toHaveURL('/settings');
await expect(page).toHaveURL(new RegExp(`/users/${id}`));
await navigateTo(page, '/settings');
await testHelpers.openPath(page, '/settings');
```

Detection rules:

- Candidate URLs must start with `/`, `http://`, or `https://`.
- Protocol-relative URLs such as `//example.com/path` are treated as external.
- Absolute URLs only count when they start with a literal Playwright `baseURL`;
  the base is stripped before route matching.
- External absolute URLs without a matching `baseURL` are ignored.
- `page.goto(...)` only contributes a URL when the first argument is a string or
  template literal.
- `page.click(...)` only contributes a URL when the selector contains an
  `href="..."` or `href='...'` value.
- Positive `.toHaveURL(...)` assertions and configured `navigationHelpers` use
  the first URL-like string literal or template literal found anywhere in the
  call arguments.
- Negative `.not.toHaveURL(...)` assertions are ignored.

## App Selector Collection

Selector coverage compares selectors declared in app JSX with selectors used by
Playwright tests. App selector source files may use these extensions:

- `.ts`
- `.tsx`
- `.js`
- `.jsx`
- `.mts`
- `.cts`
- `.mjs`
- `.cjs`

Supported JSX forms for configured `selectorAttributes`:

```tsx
<button data-testid="save" />
<button data-testid='save' />
<button data-testid={"save"} />
<button data-testid={'save'} />
<article data-testid={`user-${id}`} />
<button data-testid={id} />
```

Quoted string values and quoted expression values are exact selectors. Template
literals with at least one static part are fuzzy selectors, so `user-${id}` can
be covered by a test selector such as `user-42`.

Dynamic expressions without static template text, such as `{id}` or `` `${id}` ``,
are reported with `unsupportedDynamic: true` and never count as covered.

## Playwright Selector Detection

Detected `getByTestId(...)` forms:

```ts
await page.getByTestId('save').click();
await page.getByTestId("save").click();
await page.getByTestId(`save`).click();
await page.getByTestId(/^user-/).click();
```

`getByTestId(...)` maps to each Playwright `testIdAttribute` discovered for the
test file. By default this covers `data-testid`. If Playwright config sets
`use.testIdAttribute: 'data-pw'`, then `getByTestId('save')` covers
`data-pw="save"`.

Detected CSS attribute selectors:

```ts
await page.locator('[data-testid="save"]').click();
await page.locator("[data-testid='save']").click();
await page.locator('[data-testid^="user-"]').click();
await page.locator('[data-testid$="-button"]').click();
await page.locator('[data-testid*="nav"]').click();
```

CSS attribute selectors are detected for configured `selectorAttributes` only.
Supported operators are exact (`=`), prefix (`^=`), suffix (`$=`), and contains
(`*=`).

The selector string must appear as a string or template literal argument to a
known Playwright selector method. Supported methods include `locator`, `click`,
`fill`, `hover`, `press`, `waitForSelector`, `$`, `$$`, `$eval`, `$$eval`,
`frameLocator`, and related selector-taking methods. `dragAndDrop(...)` checks
both selector arguments.

## Matching Behavior

A Playwright selector covers an app selector only when both the attribute and
value matcher match.

- Exact app selectors match exact, prefix, suffix, contains, or regex test
  selectors according to the test selector semantics.
- Template app selectors match against their static parts.
- `getByTestId(/.../)` regex selectors are tested against static app selector
  values and against a generated sample for template selectors where each
  dynamic hole is replaced with `x`. JavaScript regex flags are ignored; the
  pattern is evaluated as a Rust regex string.
- Unsupported dynamic app selectors never count as covered.

## Limitations

- Project code is never executed.
- Non-literal optional Playwright config values are ignored.
- Computed object properties and method properties in Playwright config are not
  parsed as supported options.
- Regular-expression Playwright `testMatch` and `testIgnore` values are not
  supported; use string globs.
- URL and selector values built through variables, function calls, string
  concatenation, or conditionals are not detected unless a supported literal or
  template literal appears in the inspected call.
