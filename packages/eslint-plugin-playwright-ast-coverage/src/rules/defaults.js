"use strict";

const { rule, selectorValueNode } = require("../helpers");
const { isDefaultedPropReference } = require("../defaulted-props");
const { selectorAttributeVisitors } = require("../selector-visitor");

module.exports = rule(
  {
    type: "problem",
    docs: {
      description: "require literal defaults for prop-passed test IDs",
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
      default: "Test ID prop passthrough must have a literal default.",
    },
  },
  (context) =>
    selectorAttributeVisitors(context, (node) => {
      const valueNode = selectorValueNode(node);
      if (valueNode?.type === "Identifier" && !isDefaultedPropReference(valueNode, context)) {
        context.report({ node, messageId: "default" });
      }
    }),
);
