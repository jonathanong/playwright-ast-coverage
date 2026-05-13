"use strict";

const { isStringLiteralNode, literalString, staticTemplate } = require("./helpers");

const DEFAULTED_PROPS = new WeakMap();

function isLiteralLike(node, opts, context) {
  const value = literalString(node);
  if (value !== null) {
    return true;
  }
  if (opts.allowStaticTemplates && staticTemplate(node)) {
    return true;
  }
  if (opts.allowDefaultedProps && isDefaultedPropReference(node, context)) {
    return true;
  }
  return false;
}

function collectDefaultedProps(params) {
  const props = new Set();
  for (const param of params) {
    collectPatternDefaults(param, props);
  }
  return props;
}

function collectPatternDefaults(pattern, props) {
  if (pattern.type === "AssignmentPattern") {
    collectDefaultName(pattern.left, props, isStringLiteralNode(pattern.right));
    return;
  }
  if (pattern.type === "ObjectPattern") {
    for (const prop of pattern.properties) {
      collectObjectPropertyDefault(prop, props);
    }
  }
}

function collectObjectPropertyDefault(prop, props) {
  if (prop.type === "RestElement") {
    return;
  }
  if (prop.value.type === "AssignmentPattern") {
    collectDefaultName(prop.value.left, props, isStringLiteralNode(prop.value.right));
    return;
  }
  collectPatternDefaults(prop.value, props);
}

function collectDefaultName(node, props, hasLiteralDefault) {
  if (hasLiteralDefault && node.type === "Identifier") {
    props.add(node.name);
  }
  if (node.type === "ObjectPattern") {
    collectPatternDefaults(node, props);
  }
}

function nearestFunction(node) {
  let current = node.parent;
  while (current) {
    if (
      current.type === "FunctionDeclaration" ||
      current.type === "FunctionExpression" ||
      current.type === "ArrowFunctionExpression"
    ) {
      return current;
    }
    current = current.parent;
  }
  return null;
}

function defaultedPropsForNode(node) {
  const fn = nearestFunction(node);
  /* v8 ignore next -- direct helper tests cover the no-function return */
  if (!fn) {
    return new Set();
  }
  let props = DEFAULTED_PROPS.get(fn);
  if (!props) {
    props = collectDefaultedProps(fn.params);
    DEFAULTED_PROPS.set(fn, props);
  }
  return props;
}

function isDefaultedPropReference(node, context) {
  if (node?.type !== "Identifier" || !defaultedPropsForNode(node).has(node.name)) {
    return false;
  }
  const variable = findVariable(context.sourceCode.getScope(node), node.name);
  return Boolean(variable?.defs.some((def) => def.type === "Parameter"));
}

function findVariable(scope, name) {
  let current = scope;
  while (current) {
    const variable = current.variables.find((item) => item.name === name);
    if (variable) {
      return variable;
    }
    current = current.upper;
  }
  return null;
}

module.exports = {
  defaultedPropsForNode,
  isDefaultedPropReference,
  isLiteralLike,
};
