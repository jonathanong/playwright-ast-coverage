"use strict";

const DEFAULT_SELECTOR_ATTRIBUTES = ["data-testid", "data-pw"];
const DEFAULT_NAMING_PATTERN = "^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$";
const INTERACTIVE_ELEMENTS = new Set(["button", "input", "select", "textarea"]);
const SELECTOR_METHODS = new Set([
  "$",
  "$$",
  "$eval",
  "$$eval",
  "check",
  "click",
  "dblclick",
  "dragAndDrop",
  "fill",
  "focus",
  "frameLocator",
  "getByTestId",
  "hover",
  "locator",
  "press",
  "selectOption",
  "setInputFiles",
  "tap",
  "textContent",
  "type",
  "uncheck",
  "waitForSelector",
]);

function options(context) {
  return context.options[0] || {};
}

function selectorAttributes(option) {
  return option.selectorAttributes || DEFAULT_SELECTOR_ATTRIBUTES;
}

function canonicalAttribute(option) {
  return option.canonicalAttribute || "data-pw";
}

function isSelectorAttribute(name, attrs) {
  return attrs.includes(name);
}

function attributeName(attribute) {
  if (!attribute || attribute.type !== "JSXAttribute") {
    return null;
  }
  if (attribute.name.type !== "JSXIdentifier") {
    return null;
  }
  return attribute.name.name;
}

function literalString(node) {
  if (node.type === "Literal" && typeof node.value === "string") {
    return node.value;
  }
  if (node.type === "TemplateLiteral" && node.expressions.length === 0) {
    return node.quasis.map((quasi) => quasi.value.raw).join("");
  }
  return null;
}

function staticTemplate(node) {
  if (node && node.type === "TemplateLiteral" && node.expressions.length > 0) {
    return node.quasis.some((quasi) => quasi.value.raw.length > 0);
  }
  return false;
}

function jsxAttributeExpression(attribute) {
  if (!attribute.value) {
    return null;
  }
  if (attribute.value.type === "JSXExpressionContainer") {
    return attribute.value.expression;
  }
  return attribute.value;
}

function selectorLiteral(attribute) {
  const expression = jsxAttributeExpression(attribute);
  if (!expression) {
    return null;
  }
  return literalString(expression);
}

function callMethodName(node) {
  if (node.callee.type === "MemberExpression" && !node.callee.computed) {
    return node.callee.property.name;
  }
  if (node.callee.type === "Identifier") {
    return node.callee.name;
  }
  return null;
}

function isSelectorCall(node) {
  const name = callMethodName(node);
  return Boolean(name && SELECTOR_METHODS.has(name));
}

function cssSelectorValues(source, attrs) {
  const values = [];
  for (const attr of attrs) {
    const escaped = attr.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const regex = new RegExp(
      `\\[\\s*${escaped}\\s*([*^$]?=)\\s*(?:"([^"]*)"|'([^']*)'|([^\\s\\]]+))\\s*(?:[is])?\\s*\\]`,
      "g",
    );
    let match = regex.exec(source);
    while (match) {
      values.push({ attribute: attr, operator: match[1], value: match[2] ?? match[3] ?? match[4] });
      match = regex.exec(source);
    }
  }
  return values;
}

function isStringLiteralNode(node) {
  return literalString(node) !== null;
}

function selectorValueNode(attribute) {
  const expression = jsxAttributeExpression(attribute);
  return expression && expression.type !== "JSXEmptyExpression" ? expression : null;
}

function rule(meta, create) {
  return { meta, create };
}

module.exports = {
  DEFAULT_NAMING_PATTERN,
  attributeName,
  callMethodName,
  canonicalAttribute,
  cssSelectorValues,
  INTERACTIVE_ELEMENTS,
  isSelectorAttribute,
  isSelectorCall,
  isStringLiteralNode,
  literalString,
  options,
  rule,
  selectorAttributes,
  selectorLiteral,
  selectorValueNode,
  staticTemplate,
};
