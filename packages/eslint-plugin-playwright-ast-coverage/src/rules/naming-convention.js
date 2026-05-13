"use strict";

const { DEFAULT_NAMING_PATTERN, options, rule, selectorLiteral } = require("../helpers");
const { selectorAttributeVisitors } = require("../selector-visitor");

module.exports = rule(
  {
    type: "suggestion",
    docs: { description: "require a naming convention for literal test IDs", recommended: false },
    schema: [
      {
        type: "object",
        properties: {
          selectorAttributes: { type: "array", items: { type: "string" } },
          pattern: { type: "string" },
        },
        additionalProperties: false,
      },
    ],
    messages: { naming: "Test ID '{{value}}' does not match {{pattern}}." },
  },
  (context) => {
    const pattern = options(context).pattern || DEFAULT_NAMING_PATTERN;
    const regex = new RegExp(pattern);
    return selectorAttributeVisitors(context, (node) => {
      const value = selectorLiteral(node);
      if (value !== null && !regex.test(value)) {
        context.report({ node, messageId: "naming", data: { value, pattern } });
      }
    });
  },
);
