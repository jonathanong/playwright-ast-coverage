"use strict";

const { isFetchCall, isStaticString, rule } = require("../helpers");

module.exports = rule(
  {
    type: "problem",
    docs: {
      description: "require static fetch() URL arguments",
      recommended: true,
    },
    schema: [],
    messages: {
      dynamic:
        "fetch() URL must be a string literal or an expression-free template literal so it can be statically analyzed.",
    },
  },
  (context) => ({
    CallExpression(node) {
      if (!isFetchCall(node, context)) return;
      const url = node.arguments[0];
      if (!isStaticString(url)) {
        context.report({ node: url ?? node, messageId: "dynamic" });
      }
    },
  }),
);
