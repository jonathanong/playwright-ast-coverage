"use strict";

const { isFetchCall, isStaticString, rule } = require("../helpers");

module.exports = rule(
  {
    type: "problem",
    docs: {
      description: "require static fetch() method option",
      recommended: true,
    },
    schema: [],
    messages: {
      dynamic: "fetch() method option must be a string literal so it can be statically analyzed.",
    },
  },
  (context) => ({
    CallExpression(node) {
      if (!isFetchCall(node, context)) return;
      const opts = node.arguments[1];
      if (!opts || opts.type !== "ObjectExpression") return;
      const methodProp = opts.properties.find(
        (p) =>
          p.type === "Property" &&
          !p.computed &&
          p.key.type === "Identifier" &&
          p.key.name === "method",
      );
      if (!methodProp) return;
      if (!isStaticString(methodProp.value)) {
        context.report({ node: methodProp.value, messageId: "dynamic" });
      }
    },
  }),
);
