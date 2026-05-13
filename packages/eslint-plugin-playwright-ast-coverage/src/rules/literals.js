"use strict";

const {
  attributeName,
  callMethodName,
  isSelectorAttribute,
  options,
  rule,
  selectorAttributes,
  selectorValueNode,
} = require("../helpers");
const { isLiteralLike } = require("../defaulted-props");

module.exports = rule(
  {
    type: "problem",
    docs: {
      description: "require literal Playwright test IDs",
      recommended: true,
    },
    schema: [
      {
        type: "object",
        properties: {
          selectorAttributes: { type: "array", items: { type: "string" } },
          allowDefaultedProps: { type: "boolean" },
          allowStaticTemplates: { type: "boolean" },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      literal: "Use a literal test ID, a static template, or a prop with a literal default.",
    },
  },
  (context) => {
    const opts = {
      allowDefaultedProps: options(context).allowDefaultedProps !== false,
      allowStaticTemplates: options(context).allowStaticTemplates === true,
    };
    const attrs = selectorAttributes(options(context));
    return {
      JSXAttribute(node) {
        const name = attributeName(node);
        if (!name || !isSelectorAttribute(name, attrs)) {
          return;
        }
        const valueNode = selectorValueNode(node);
        if (!valueNode || !isLiteralLike(valueNode, opts, context)) {
          context.report({ node, messageId: "literal" });
        }
      },
      CallExpression(node) {
        if (callMethodName(node) !== "getByTestId") {
          return;
        }
        const arg = node.arguments[0];
        if (!arg || !isLiteralLike(arg, opts, context)) {
          context.report({ node: arg || node, messageId: "literal" });
        }
      },
    };
  },
);
