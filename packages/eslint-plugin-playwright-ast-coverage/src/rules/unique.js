"use strict";

const { rule, selectorLiteral } = require("../helpers");
const { selectorAttributeVisitors } = require("../selector-visitor");

module.exports = rule(
  {
    type: "problem",
    docs: {
      description: "require unique literal test IDs within a file",
      recommended: true,
    },
    schema: [
      {
        type: "object",
        properties: { selectorAttributes: { type: "array", items: { type: "string" } } },
        additionalProperties: false,
      },
    ],
    messages: {
      duplicate: "Test ID '{{value}}' is already used in this file.",
    },
  },
  (context) => {
    const seen = new Map();
    return selectorAttributeVisitors(context, (node) => {
      const value = selectorLiteral(node);
      /* v8 ignore next -- covered by non-literal selector values in rule tests */
      if (value === null) {
        return;
      }
      if (seen.has(value)) {
        context.report({ node, messageId: "duplicate", data: { value } });
      } else {
        seen.set(value, node);
      }
    });
  },
);
