"use strict";

function rule(meta, create) {
  return { meta, create };
}

function isStaticString(node) {
  if (!node) return false;
  if (node.type === "Literal") return typeof node.value === "string";
  if (node.type === "TemplateLiteral") return node.expressions.length === 0;
  return false;
}

function isFetchShadowed(scope) {
  while (scope) {
    const variable = scope.variables.find((v) => v.name === "fetch");
    if (variable) return scope.type !== "global";
    scope = scope.upper;
  }
  return false;
}

function isFetchCall(node, context) {
  if (node.callee.type !== "Identifier" || node.callee.name !== "fetch") return false;
  return !isFetchShadowed(context.sourceCode.getScope(node));
}

module.exports = { isFetchCall, isStaticString, rule };
