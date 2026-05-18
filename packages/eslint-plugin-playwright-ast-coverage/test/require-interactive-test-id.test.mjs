import assert from "node:assert/strict";
import { describe, it } from "vitest";
import { messages } from "./helpers.mjs";

describe("require-interactive-test-id", () => {
  it("reports native interactive elements without a test ID", () => {
    assert.deepEqual(
      messages("<><button /><input /><select /><textarea /></>;", "require-interactive-test-id"),
      ["missing", "missing", "missing", "missing"],
    );
  });

  it("reports anchor elements with href without a test ID", () => {
    assert.deepEqual(messages("<><a href='/x' /><a>link</a></>;", "require-interactive-test-id"), [
      "missing",
    ]);
  });

  it("reports elements with onClick without a test ID", () => {
    assert.deepEqual(messages("<div onClick={() => {}} />;", "require-interactive-test-id"), [
      "missing",
    ]);
  });

  it("reports elements with interactive roles without a test ID", () => {
    const roles = [
      "button",
      "checkbox",
      "link",
      "menuitem",
      "option",
      "radio",
      "switch",
      "tab",
      "textbox",
    ];
    const code = "<>" + roles.map((r) => `<div role="${r}" />`).join("") + "</>;";
    assert.deepEqual(
      messages(code, "require-interactive-test-id"),
      roles.map(() => "missing"),
    );
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
      ["missing"],
    );
  });
});
