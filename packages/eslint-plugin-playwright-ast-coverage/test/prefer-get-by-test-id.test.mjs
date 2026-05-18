import { RuleTester } from "eslint";
import { describe, it } from "vitest";
import rule from "../src/rules/prefer-get-by-test-id.js";

RuleTester.describe = describe;
RuleTester.it = it;

const ruleTester = new RuleTester({
  languageOptions: { ecmaVersion: 2024, sourceType: "module" },
});

ruleTester.run("prefer-get-by-test-id", rule, {
  valid: [
    { code: 'page.getByTestId("save")' },
    { code: 'page[method]("[data-pw=\\"computed\\"]")' },
    { code: 'page.locator()' },
    { code: 'page.locator(selector)' },
    { code: 'page.locator("")' },
    { code: 'page.click("[data-testid^=\\"user-\\"]")' },
    { code: 'const selector = "[data-pw=\\"ignored\\"]"' },
    { code: 'page.locator(`${dynamic}`)' },
    { code: 'page.locator(123)' },
    { code: 'page.locator(`[data-testid="${dynamic}"]`)' },
    { code: 'page.locator(null)' },
    { code: 'page.locator(selector, "[data-pw=\\"ignored\\"]")' },
    { code: 'page.dragAndDrop(selector, selector)' },
    { code: 'page.locator("[data-pw=\\"save\\"]", { hasText: "foo" })', options: [{ selectorAttributes: ["data-qa"] }] },
    { code: 'page.locator(a, b)' },
    { code: 'page.locator(``)' },
  ],
  invalid: [
    {
      code: 'locator("[data-pw=\\"direct\\"]")',
      errors: [{ messageId: "prefer", data: { value: "direct" } }],
    },
    {
      code: 'page.locator("[data-pw=\\"save\\"]")',
      errors: [{ messageId: "prefer", data: { value: "save" } }],
    },
    {
      code: 'page.locator("[data-pw=\'open\']")',
      errors: [{ messageId: "prefer", data: { value: "open" } }],
    },
    {
      code: 'page.locator("[ data-pw = unquoted ]")',
      errors: [{ messageId: "prefer", data: { value: "unquoted" } }],
    },
    {
      code: 'page.locator("[data-pw=flagged i]")',
      errors: [{ messageId: "prefer", data: { value: "flagged" } }],
    },
    {
      code: 'page.dragAndDrop("[data-testid=\\"source\\"]", "[data-pw=\\"target\\"]")',
      errors: [
        { messageId: "prefer", data: { value: "source" } },
        { messageId: "prefer", data: { value: "target" } },
      ],
    },
    {
      code: 'page.locator("[data-pw=\\"save\\"]", { hasText: "foo" })',
      errors: [{ messageId: "prefer", data: { value: "save" } }],
    },
    {
      code: 'page.locator("invalid", { has: page.locator("[data-pw=\\"ignored\\"]") })',
      errors: [{ messageId: "prefer", data: { value: "ignored" } }],
    },
    {
      code: 'page.dragAndDrop(selector, "[data-pw=\\"target\\"]")',
      errors: [
        { messageId: "prefer", data: { value: "target" } },
      ],
    },
    {
      code: 'page.dragAndDrop("[data-pw=\\"source\\"]", selector)',
      errors: [
        { messageId: "prefer", data: { value: "source" } },
      ],
    },
    {
      code: 'page.locator("[data-qa=\\"save\\"]")',
      options: [{ selectorAttributes: ["data-qa"] }],
      errors: [{ messageId: "prefer", data: { value: "save" } }],
    },
  ],
});
