import { RuleTester } from "eslint";
import assert from "node:assert/strict";
import { afterAll, describe, it } from "vitest";
import rule from "../src/rules/require-interactive-test-id.js";
import { messages } from "./helpers.mjs";

RuleTester.describe = describe;
RuleTester.it = it;
RuleTester.itOnly = it.only;
RuleTester.afterAll = afterAll;

const tester = new RuleTester({
  languageOptions: {
    ecmaVersion: 2024,
    sourceType: "module",
    parserOptions: { ecmaFeatures: { jsx: true } },
  },
});

tester.run("require-interactive-test-id", rule, {
  valid: [
    { code: "<a />;" },
    { code: "<Foo.bar data-testid='id' />;" },
    { code: "<a onClick={fn} href='/x' data-testid='id' />;" },
    { code: "<div onClick={fn} data-testid='id' />;" },
    { code: "<div role='button' data-testid='id' />;" },
    { code: "<div role='presentation' />;" },
    { code: "<div {...props} />;" },
    { code: "<Comp.Button />;" },
    { code: "<a>link</a>;" },
    { code: "<div role='button' data-pw={id} />;" },
    {
      code: `<>
      <button data-pw="save" />
      <input data-testid="input" />
      <a href="/x" data-pw="link" />
      <div onClick={fn} data-pw="click" />
      <div role="button" data-pw="btn" />
    </>`,
    },
    {
      code: `<button data-qa="save" />`,
      options: [{ selectorAttributes: ["data-qa"] }],
    },
  ],
  invalid: [
    {
      code: "<div role='button' />;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: "<Comp.Button onClick={fn} />;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: "<div onClick={fn} role='presentation' />;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: "<a href='/x' />;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: "<Foo.Bar onClick={handler} />;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: "<><button /><input /><select /><textarea /></>;",
      errors: [
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
      ],
    },
    {
      code: "<><a href='/x' /><a>link</a></>;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: "<div onClick={() => {}} />;",
      errors: [{ messageId: "missing" }],
    },
    {
      code: [
        "<>",
        '  <div role="button" />',
        '  <div role="checkbox" />',
        '  <div role="link" />',
        '  <div role="menuitem" />',
        '  <div role="option" />',
        '  <div role="radio" />',
        '  <div role="switch" />',
        '  <div role="tab" />',
        '  <div role="textbox" />',
        "</>",
      ].join("\n"),
      errors: [
        { messageId: "missing", line: 2, column: 4 },
        { messageId: "missing", line: 3, column: 4 },
        { messageId: "missing", line: 4, column: 4 },
        { messageId: "missing", line: 5, column: 4 },
        { messageId: "missing", line: 6, column: 4 },
        { messageId: "missing", line: 7, column: 4 },
        { messageId: "missing", line: 8, column: 4 },
        { messageId: "missing", line: 9, column: 4 },
        { messageId: "missing", line: 10, column: 4 },
      ],
    },
    {
      code: '<>\n<button data-qa="save" />\n<button data-pw="save" />\n</>',
      options: [{ selectorAttributes: ["data-qa"] }],
      errors: [{ messageId: "missing", line: 3, column: 2 }],
    },
  ],
});

describe("messages coverage", () => {
  it("reports anchor href and onClick controls without selectors", () => {
    assert.deepEqual(messages("<a href='/x' />;", "require-interactive-test-id"), ["missing"]);
    assert.deepEqual(messages("<div onClick={fn} />;", "require-interactive-test-id"), ["missing"]);
  });

  it("ignores non-interactive role without onClick", () => {
    assert.deepEqual(messages("<div role='presentation' />;", "require-interactive-test-id"), []);
  });
});
