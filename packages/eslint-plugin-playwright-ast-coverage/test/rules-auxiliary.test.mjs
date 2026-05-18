import assert from "node:assert/strict";
import { describe, it } from "vitest";
import { fixture, lint, messages, plugin } from "./helpers.mjs";

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
