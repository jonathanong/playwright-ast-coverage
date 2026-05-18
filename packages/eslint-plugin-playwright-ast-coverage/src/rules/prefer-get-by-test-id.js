"use strict";

const {
  callMethodName,
  cssSelectorValues,
  isSelectorCall,
  literalString,
  options,
  rule,
  selectorAttributes,
} = require("../helpers");

module.exports = rule(
  {
    type: "suggestion",
    docs: { description: "prefer getByTestId over CSS test-id selectors", recommended: false },
    schema: [
      {
        type: "object",
        properties: { selectorAttributes: { type: "array", items: { type: "string" } } },
        additionalProperties: false,
      },
    ],
    messages: { prefer: "Prefer getByTestId('{{value}}') for exact test ID selectors." },
  },
  (context) => {
    const attrs = selectorAttributes(options(context));
    return {
      CallExpression(node) {
        const methodName = callMethodName(node);
        if (!isSelectorCall(node) || methodName === "getByTestId") {
          return;
        }
        for (const arg of node.arguments.slice(0, methodName === "dragAndDrop" ? 2 : 1)) {
          const source = literalString(arg);
          if (typeof source !== "string" || source === "") {
            continue;
          }
          for (const selector of cssSelectorValues(source, attrs)) {
            if (selector.operator === "=") {
              context.report({ node: arg, messageId: "prefer", data: { value: selector.value } });
            }
          }
        }
      },
    };
  },
);
