"use strict";

const {
  attributeName,
  INTERACTIVE_ELEMENTS,
  isSelectorAttribute,
  options,
  rule,
  selectorAttributes,
  selectorLiteral,
} = require("../helpers");

module.exports = rule(
  {
    type: "suggestion",
    docs: { description: "require test IDs on interactive JSX elements", recommended: false },
    schema: [
      {
        type: "object",
        properties: { selectorAttributes: { type: "array", items: { type: "string" } } },
        additionalProperties: false,
      },
    ],
    messages: { missing: "Interactive elements should have a test ID." },
  },
  (context) => {
    const attrs = selectorAttributes(options(context));
    return {
      JSXOpeningElement(node) {
        const elementName = node.name.type === "JSXIdentifier" ? node.name.name : null;
        const hasSelector = node.attributes.some((attr) =>
          isSelectorAttribute(attributeName(attr), attrs),
        );
        if (hasSelector || !isInteractiveElement(elementName, node.attributes)) {
          return;
        }
        context.report({ node: node.name, messageId: "missing" });
      },
    };
  },
);

function isInteractiveElement(elementName, attributes) {
  if (INTERACTIVE_ELEMENTS.has(elementName)) {
    return true;
  }
  /* v8 ignore next -- covered behavior is asserted through require-interactive-test-id */
  if (elementName === "a" && attributes.some((attr) => attributeName(attr) === "href")) {
    return true;
  }
  return attributes.some((attr) => {
    if (attributeName(attr) === "onClick") {
      return true;
    }
    if (attributeName(attr) !== "role") {
      return false;
    }
    return [
      "button",
      "checkbox",
      "link",
      "menuitem",
      "option",
      "radio",
      "switch",
      "tab",
      "textbox",
    ].includes(selectorLiteral(attr));
  });
}
