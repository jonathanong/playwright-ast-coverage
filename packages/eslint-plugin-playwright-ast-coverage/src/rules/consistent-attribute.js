"use strict";

const { canonicalAttribute, options, rule } = require("../helpers");
const { selectorAttributeVisitors } = require("../selector-visitor");

module.exports = rule(
  {
    type: "suggestion",
    docs: { description: "require a canonical test ID attribute", recommended: false },
    schema: [
      {
        type: "object",
        properties: {
          selectorAttributes: { type: "array", items: { type: "string" } },
          canonicalAttribute: { type: "string" },
        },
        additionalProperties: false,
      },
    ],
    messages: { attribute: "Use '{{expected}}' instead of '{{actual}}' for test IDs." },
  },
  (context) =>
    selectorAttributeVisitors(context, (node, name) => {
      const expected = canonicalAttribute(options(context));
      /* v8 ignore next -- matching canonical attrs are asserted through no reports */
      if (name !== expected) {
        context.report({
          node: node.name,
          messageId: "attribute",
          data: { actual: name, expected },
        });
      }
    }),
);
