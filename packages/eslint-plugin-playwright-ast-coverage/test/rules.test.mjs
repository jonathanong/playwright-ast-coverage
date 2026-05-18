import assert from "node:assert/strict";
import { describe, it } from "vitest";
import { fixture, lint, messages, plugin } from "./helpers.mjs";

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

  it("accepts const destructured literal defaults", () => {
    const code = `function A(props) { const { "data-pw": dataPw = "save", nested: { nestedPw = "open" } = {} } = props; const [, arrayPw = "array"] = props.items; page.getByTestId(dataPw); if (props.ready) { const { readyPw = "ready" } = props; return <button data-pw={readyPw} />; } return <><button data-pw={dataPw} /><button data-pw={nestedPw} /><button data-pw={arrayPw} /></>; }`;
    assert.deepEqual(messages(code, "literals"), []);
  });

  it("rejects unsafe const destructured defaults", () => {
    const code = `function A(props) { function B() { const { "data-pw": inner = "inner" } = props; } const { "data-pw": missing } = props; const { "data-testid": dynamic = id } = props; let { "data-qa": mutable = "open" } = props; return <><button data-pw={inner} /><button data-pw={missing} /><button data-pw={dynamic} /><button data-pw={mutable} /></>; }`;
    assert.deepEqual(messages(code, "literals"), ["literal", "literal", "literal", "literal"]);
  });

  it("does not treat rest bindings as defaulted props", () => {
    const code = `function A(props) { const { ...rest } = props; return <button data-pw={rest} />; }`;
    assert.deepEqual(messages(code, "literals"), ["literal"]);
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

  it("accepts const destructured literal defaults", () => {
    const code = `function A(props) { const { "data-pw": dataPw = "save" } = props; return <button data-pw={dataPw} />; }`;
    assert.deepEqual(messages(code, "defaults"), []);
  });

  it("rejects shadowed and unsafe const destructured defaults", () => {
    const code = `function A(props) { const { "data-pw": dataPw = "save" } = props; const { "data-testid": dynamic = id } = props; let { "data-qa": mutable = "open" } = props; { const dataPw = id; return <><button data-pw={dataPw} /><button data-pw={dynamic} /><button data-pw={mutable} /></>; } }`;
    assert.deepEqual(messages(code, "defaults"), ["default", "default", "default"]);
  });

  it("rejects const defaults from non-props, before declaration, and mismatched parameters", () => {
    const code = `function A(props, cfg) { const { "data-pw": fromCfg = "cfg" } = cfg; const before = <button data-pw={late} />; const { "data-pw": late = "late" } = props; return <><button data-pw={fromCfg} />{before}</>; } function B(testId, props) { { const { "data-pw": testId = "save" } = props; } return <button data-pw={testId} />; }`;
    assert.deepEqual(messages(code, "defaults"), ["default", "default", "default"]);
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
  it("reports native interactive elements without a test ID", () => {
    assert.deepEqual(messages("<><button /><input /><select /><textarea /></>;", "require-interactive-test-id"), [
      "missing", "missing", "missing", "missing"
    ]);
  });

  it("reports anchor elements with href without a test ID", () => {
    assert.deepEqual(messages("<><a href='/x' /><a>link</a></>;", "require-interactive-test-id"), [
      "missing"
    ]);
  });

  it("reports elements with onClick without a test ID", () => {
    assert.deepEqual(messages("<div onClick={() => {}} />;", "require-interactive-test-id"), [
      "missing"
    ]);
  });

  it("reports elements with interactive roles without a test ID", () => {
    const roles = ["button", "checkbox", "link", "menuitem", "option", "radio", "switch", "tab", "textbox"];
    const code = "<>" + roles.map(r => `<div role="${r}" />`).join("") + "</>;";
    assert.deepEqual(messages(code, "require-interactive-test-id"), roles.map(() => "missing"));
  });

  it("does not report elements with non-interactive roles", () => {
    assert.deepEqual(messages("<div role='presentation' />;", "require-interactive-test-id"), []);
  });

  it("handles non-jsx attributes gracefully", () => {
    assert.deepEqual(messages("<div {...props} />;", "require-interactive-test-id"), []);
  });

  it("handles non-JSXIdentifier element names gracefully", () => {
    assert.deepEqual(messages("<Comp.Button />;", "require-interactive-test-id"), []);
  });

  it("does not report interactive elements with a default test ID", () => {
    const code = `<>
      <button data-pw="save" />
      <input data-testid="input" />
      <a href="/x" data-pw="link" />
      <div onClick={fn} data-pw="click" />
      <div role="button" data-pw="btn" />
    </>`;
    assert.deepEqual(messages(code, "require-interactive-test-id"), []);
  });

  it("respects the selectorAttributes option", () => {
    const code = `<>
      <button data-qa="save" />
      <button data-pw="save" />
    </>`;
    assert.deepEqual(
      messages(code, "require-interactive-test-id", { selectorAttributes: ["data-qa"] }),
      ["missing"]
    );
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
