import assert from "node:assert/strict";
import { describe, it } from "vitest";
import { fixture, lint, messages, plugin, require } from "./helpers.mjs";

describe("plugin exports", () => {
  it("exposes rules and flat configs", () => {
    assert.equal(plugin.meta.name, "eslint-plugin-playwright-ast-coverage");
    assert.ok(plugin.rules.literals);
    assert.equal(plugin.configs.recommended.rules["playwright-ast-coverage/literals"], "error");
    assert.deepEqual(plugin.configs.strict.rules["playwright-ast-coverage/consistent-attribute"], [
      "error",
      { canonicalAttribute: "data-pw" },
    ]);
  });
});

describe("defaulted prop helpers", () => {
  it("returns an empty set outside functions", () => {
    const { defaultedPropsForNode } = require("../src/defaulted-props");
    assert.equal(defaultedPropsForNode({ parent: null }).size, 0);
  });
});

describe("literals", () => {
  it("accepts literals, expression literals, empty static templates, static templates, and defaulted props", () => {
    assert.deepEqual(
      messages(fixture("literals-valid.jsx"), "literals", { allowStaticTemplates: true }),
      [],
    );
  });

  it("reports missing, dynamic, non-defaulted, and forbidden template values", () => {
    assert.deepEqual(messages(fixture("literals-invalid.jsx"), "literals"), [
      "literal",
      "literal",
      "literal",
      "literal",
      "literal",
      "literal",
    ]);
  });

  it("rejects templates without static text and allows defaulted identifiers outside props", () => {
    const code = `
      function A(testId = "save") {
        helper();
        page.getByTestId(testId);
        return <button data-pw={\`\${id}\`} />;
      }
    `;
    assert.deepEqual(messages(code, "literals", { allowStaticTemplates: true }), ["literal"]);
  });

  it("can be configured for literal-only mode and custom attributes", () => {
    const code = `
      const A = ({ testId = "save" }) => <button data-qa={testId} />;
    `;
    assert.deepEqual(
      messages(code, "literals", {
        selectorAttributes: ["data-qa"],
        allowDefaultedProps: false,
      }),
      ["literal"],
    );
  });

  it("does not treat shadowed locals as defaulted props", () => {
    const code = `
      function A({ testId = "save" }) {
        {
          const testId = id;
          return <button data-pw={testId} />;
        }
      }
    `;
    assert.deepEqual(messages(code, "literals"), ["literal"]);
  });

  it("accepts defaulted props through nested scopes only when unshadowed", () => {
    const code = `
      function A({ testId = "save" }) {
        {
          return <button data-pw={testId} />;
        }
      }
    `;
    assert.deepEqual(messages(code, "literals"), []);
  });

  it("rejects missing getByTestId arguments and identifiers outside functions", () => {
    assert.deepEqual(messages("page.getByTestId(); page.getByTestId(testId);", "literals"), [
      "literal",
      "literal",
    ]);
  });
});

describe("defaults", () => {
  it("requires literal defaults for prop passthrough", () => {
    assert.deepEqual(messages(fixture("defaults.jsx"), "defaults"), ["default", "default"]);
  });

  it("rejects shadowed defaulted props", () => {
    const code = `
      function A({ testId = "save" }) {
        {
          const testId = id;
          return <button data-pw={testId} />;
        }
      }
    `;
    assert.deepEqual(messages(code, "defaults"), ["default"]);
  });
});

describe("unique", () => {
  it("reports duplicate exact literals within a file", () => {
    assert.deepEqual(messages(fixture("unique.jsx"), "unique"), ["duplicate", "duplicate"]);
    assert.deepEqual(messages("<button data-pw={id} />;", "unique"), []);
  });
});

describe("no-empty", () => {
  it("reports empty literal test IDs", () => {
    assert.deepEqual(
      messages(
        "<><button data-pw='' /><button data-pw /><button data-pw={'ok'} /></>;",
        "no-empty",
      ),
      ["empty"],
    );
  });
});

describe("consistent-attribute", () => {
  it("requires the configured canonical attribute", () => {
    assert.deepEqual(messages("<button data-testid='save' />;", "consistent-attribute"), [
      "attribute",
    ]);
    assert.deepEqual(messages("<button data-pw='save' />;", "consistent-attribute"), []);
    assert.deepEqual(
      messages("<button data-qa='save' />;", "consistent-attribute", {
        selectorAttributes: ["data-qa"],
        canonicalAttribute: "data-qa",
      }),
      [],
    );
  });
});

describe("require-interactive-test-id", () => {
  it("reports interactive elements without a test ID", () => {
    assert.equal(messages(fixture("interactive.jsx"), "require-interactive-test-id").length, 15);
  });
});

describe("prefer-get-by-test-id", () => {
  it("reports exact CSS test-id selectors in Playwright selector calls", () => {
    assert.deepEqual(messages(fixture("prefer-get-by-testid.js"), "prefer-get-by-test-id"), [
      "prefer",
      "prefer",
      "prefer",
      "prefer",
      "prefer",
      "prefer",
      "prefer",
    ]);
  });
});

describe("naming-convention", () => {
  it("checks literal values against a configurable pattern", () => {
    assert.deepEqual(
      messages(
        "<><button data-pw='SaveButton' /><button data-pw='save-button' /></>;",
        "naming-convention",
      ),
      ["naming"],
    );
    assert.deepEqual(
      messages("<button data-pw='SaveButton' />;", "naming-convention", {
        pattern: "^[A-Z][A-Za-z]+$",
      }),
      [],
    );
  });
});

describe("strict config", () => {
  it("runs the strict rule set", () => {
    const messages = lint("<button data-testid='Save' />;", plugin.configs.strict.rules);
    assert.deepEqual(messages.map((message) => message.ruleId).sort(), [
      "playwright-ast-coverage/consistent-attribute",
      "playwright-ast-coverage/naming-convention",
    ]);
  });
});
