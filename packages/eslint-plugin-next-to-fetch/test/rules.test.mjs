import assert from "node:assert/strict";
import { describe, it } from "vitest";
import { lint, messages, plugin } from "./helpers.mjs";

describe("plugin exports", () => {
  it("exposes rules and flat configs", () => {
    assert.equal(plugin.meta.name, "eslint-plugin-next-to-fetch");
    assert.ok(plugin.rules["static-fetch-url"]);
    assert.ok(plugin.rules["static-fetch-method"]);
    assert.equal(plugin.configs.recommended.rules["next-to-fetch/static-fetch-url"], "error");
    assert.equal(plugin.configs.recommended.rules["next-to-fetch/static-fetch-method"], "error");
  });
});

describe("static-fetch-url", () => {
  it("accepts string literal URLs", () => {
    assert.deepEqual(messages("fetch('https://api.example.com/users');", "static-fetch-url"), []);
    assert.deepEqual(messages('fetch("https://api.example.com/users");', "static-fetch-url"), []);
  });

  it("accepts expression-free template literals", () => {
    assert.deepEqual(messages("fetch(`https://api.example.com/users`);", "static-fetch-url"), []);
  });

  it("accepts fetch with static URL and options", () => {
    assert.deepEqual(
      messages(
        "fetch('https://api.example.com/users', { cache: 'force-cache' });",
        "static-fetch-url",
      ),
      [],
    );
  });

  it("reports identifier URLs", () => {
    assert.deepEqual(messages("fetch(url);", "static-fetch-url"), ["dynamic"]);
  });

  it("reports template literal URLs with expressions", () => {
    assert.deepEqual(messages("fetch(`https://api.example.com/${id}`);", "static-fetch-url"), [
      "dynamic",
    ]);
  });

  it("reports call expression URLs", () => {
    assert.deepEqual(messages("fetch(getUrl());", "static-fetch-url"), ["dynamic"]);
  });

  it("reports binary expression URLs", () => {
    assert.deepEqual(messages("fetch(base + path);", "static-fetch-url"), ["dynamic"]);
  });

  it("reports missing URL argument", () => {
    assert.deepEqual(messages("fetch();", "static-fetch-url"), ["dynamic"]);
  });

  it("does not report when fetch is shadowed by a parameter", () => {
    assert.deepEqual(messages("function f(fetch) { fetch(url); }", "static-fetch-url"), []);
  });

  it("does not report when fetch is shadowed by a local variable", () => {
    assert.deepEqual(messages("const fetch = mockFetch; fetch(url);", "static-fetch-url"), []);
  });

  it("does not report on non-fetch call expressions", () => {
    assert.deepEqual(messages("request('https://api.example.com/users');", "static-fetch-url"), []);
  });

  it("does not report on method calls named fetch", () => {
    assert.deepEqual(
      messages("client.fetch('https://api.example.com/users');", "static-fetch-url"),
      [],
    );
  });

  it("does not treat fetch configured as a global as shadowed", () => {
    assert.deepEqual(messages("fetch(url);", "static-fetch-url", { fetch: "readonly" }), [
      "dynamic",
    ]);
  });
});

describe("static-fetch-method", () => {
  it("accepts fetch without options", () => {
    assert.deepEqual(messages("fetch('https://api.example.com');", "static-fetch-method"), []);
  });

  it("accepts fetch with empty options", () => {
    assert.deepEqual(messages("fetch('https://api.example.com', {});", "static-fetch-method"), []);
  });

  it("accepts fetch with string literal method", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { method: 'POST' });", "static-fetch-method"),
      [],
    );
    assert.deepEqual(
      messages('fetch("https://api.example.com", { method: "GET" });', "static-fetch-method"),
      [],
    );
  });

  it("accepts fetch with expression-free template method", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { method: `POST` });", "static-fetch-method"),
      [],
    );
  });

  it("accepts fetch with no method property", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { cache: 'no-store' });", "static-fetch-method"),
      [],
    );
  });

  it("accepts fetch with non-object second argument", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', opts);", "static-fetch-method"),
      [],
    );
  });

  it("accepts fetch with spread-only options", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { ...opts });", "static-fetch-method"),
      [],
    );
  });

  it("accepts fetch with computed method key", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { ['method']: 'GET' });", "static-fetch-method"),
      [],
    );
  });

  it("accepts fetch with string literal method key and literal value", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { 'method': 'POST' });", "static-fetch-method"),
      [],
    );
  });

  it("reports string literal method key with non-literal value", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { 'method': verb });", "static-fetch-method"),
      ["dynamic"],
    );
  });

  it("reports identifier method values", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { method: method });", "static-fetch-method"),
      ["dynamic"],
    );
  });

  it("reports call expression method values", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { method: getMethod() });", "static-fetch-method"),
      ["dynamic"],
    );
  });

  it("reports template literal method with expressions", () => {
    assert.deepEqual(
      messages("fetch('https://api.example.com', { method: `${verb}` });", "static-fetch-method"),
      ["dynamic"],
    );
  });

  it("does not report when fetch is shadowed", () => {
    assert.deepEqual(
      messages("function f(fetch) { fetch('url', { method: verb }); }", "static-fetch-method"),
      [],
    );
  });
});

describe("recommended config", () => {
  it("runs the recommended rule set", () => {
    const results = lint("fetch(url);", plugin.configs.recommended.rules);
    const ruleIds = results.map((m) => m.ruleId).sort();
    assert.deepEqual(ruleIds, ["next-to-fetch/static-fetch-url"]);
  });
});
