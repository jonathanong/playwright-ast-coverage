import { RuleTester } from "eslint";
import { describe, it } from "vitest";
import rule from "../src/rules/require-interactive-test-id.js";

RuleTester.describe = describe;
RuleTester.it = it;

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
      code: `<>
        <div role="button" />
        <div role="checkbox" />
        <div role="link" />
        <div role="menuitem" />
        <div role="option" />
        <div role="radio" />
        <div role="switch" />
        <div role="tab" />
        <div role="textbox" />
      </>`,
      errors: [
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
        { messageId: "missing" },
      ],
    },
    {
      code: `<>
        <button data-qa="save" />
        <button data-pw="save" />
      </>`,
      options: [{ selectorAttributes: ["data-qa"] }],
      errors: [{ messageId: "missing" }],
    },
  ],
});
