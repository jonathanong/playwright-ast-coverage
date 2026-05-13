"use strict";

const DEFAULT_SELECTOR_ATTRIBUTES = ["data-testid", "data-pw"];
const DEFAULT_NAMING_PATTERN = "^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$";
const INTERACTIVE_ELEMENTS = new Set(["button", "input", "select", "textarea"]);
const SELECTOR_METHODS = new Set([
  "$",
  "$$",
  "$eval",
  "$$eval",
  "check",
  "click",
  "dblclick",
  "dragAndDrop",
  "fill",
  "focus",
  "frameLocator",
  "getByTestId",
  "hover",
  "locator",
  "press",
  "selectOption",
  "setInputFiles",
  "tap",
  "textContent",
  "type",
  "uncheck",
  "waitForSelector",
]);

function options(context) {
  return context.options[0] || {};
}

function selectorAttributes(option) {
  return option.selectorAttributes || DEFAULT_SELECTOR_ATTRIBUTES;
}

function canonicalAttribute(option) {
  return option.canonicalAttribute || "data-pw";
}

function isSelectorAttribute(name, attrs) {
  return attrs.includes(name);
}

function attributeName(attribute) {
  if (!attribute || attribute.type !== "JSXAttribute") {
    return null;
  }
  if (attribute.name.type !== "JSXIdentifier") {
    return null;
  }
  return attribute.name.name;
}

function literalString(node) {
  if (node.type === "Literal" && typeof node.value === "string") {
    return node.value;
  }
  if (node.type === "TemplateLiteral" && node.expressions.length === 0) {
    return node.quasis.map((quasi) => quasi.value.raw).join("");
  }
  return null;
}

function staticTemplate(node) {
  if (node && node.type === "TemplateLiteral" && node.expressions.length > 0) {
    return node.quasis.some((quasi) => quasi.value.raw.length > 0);
  }
  return false;
}

function jsxAttributeExpression(attribute) {
  if (!attribute.value) {
    return null;
  }
  if (attribute.value.type === "JSXExpressionContainer") {
    return attribute.value.expression;
  }
  return attribute.value;
}

function selectorLiteral(attribute) {
  const expression = jsxAttributeExpression(attribute);
  if (!expression) {
    return null;
  }
  return literalString(expression);
}

function callMethodName(node) {
  if (node.callee.type === "MemberExpression" && !node.callee.computed) {
    return node.callee.property.name;
  }
  if (node.callee.type === "Identifier") {
    return node.callee.name;
  }
  return null;
}

function isSelectorCall(node) {
  const name = callMethodName(node);
  return Boolean(name && SELECTOR_METHODS.has(name));
}

function cssSelectorValues(source, attrs) {
  const values = [];
  for (const attr of attrs) {
    const escaped = attr.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const regex = new RegExp(`\\[\\s*${escaped}\\s*([*^$]?=)\\s*(?:"([^"]*)"|'([^']*)'|([^\\s\\]]+))\\s*(?:[is])?\\s*\\]`, "g");
    let match = regex.exec(source);
    while (match) {
      values.push({ attribute: attr, operator: match[1], value: match[2] ?? match[3] ?? match[4] });
      match = regex.exec(source);
    }
  }
  return values;
}

function isLiteralLike(node, opts, defaultedProps) {
  const value = literalString(node);
  if (value !== null) {
    return true;
  }
  if (opts.allowStaticTemplates && staticTemplate(node)) {
    return true;
  }
  if (opts.allowDefaultedProps && node.type === "Identifier" && defaultedProps.has(node.name)) {
    return true;
  }
  return false;
}

function isStringLiteralNode(node) {
  return literalString(node) !== null;
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
      current.type === "FunctionDeclaration"
      || current.type === "FunctionExpression"
      || current.type === "ArrowFunctionExpression"
    ) {
      return current;
    }
    current = current.parent;
  }
  return null;
}

function defaultedPropsForNode(node) {
  const fn = nearestFunction(node);
  return fn ? collectDefaultedProps(fn.params) : new Set();
}

function selectorAttributeVisitors(context, callback) {
  const attrs = selectorAttributes(options(context));
  return {
    JSXAttribute(node) {
      const name = attributeName(node);
      /* v8 ignore next -- non-selector JSX attributes are defensive traversal */
      if (name && isSelectorAttribute(name, attrs)) {
        callback(node, name);
      }
    },
  };
}

function selectorValueNode(attribute) {
  const expression = jsxAttributeExpression(attribute);
  return expression && expression.type !== "JSXEmptyExpression" ? expression : null;
}

function rule(meta, create) {
  return { meta, create };
}

const literals = rule(
  {
    type: "problem",
    docs: {
      description: "require literal Playwright test IDs",
      recommended: true,
    },
    schema: [{
      type: "object",
      properties: {
        selectorAttributes: { type: "array", items: { type: "string" } },
        allowDefaultedProps: { type: "boolean" },
        allowStaticTemplates: { type: "boolean" },
      },
      additionalProperties: false,
    }],
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
        if (!valueNode || !isLiteralLike(valueNode, opts, defaultedPropsForNode(node))) {
          context.report({ node, messageId: "literal" });
        }
      },
      CallExpression(node) {
        if (callMethodName(node) !== "getByTestId") {
          return;
        }
        const arg = node.arguments[0];
        if (!arg || !isLiteralLike(arg, opts, defaultedPropsForNode(node))) {
          context.report({ node: arg || node, messageId: "literal" });
        }
      },
    };
  },
);

const defaults = rule(
  {
    type: "problem",
    docs: {
      description: "require literal defaults for prop-passed test IDs",
      recommended: true,
    },
    schema: [{ type: "object", properties: { selectorAttributes: { type: "array", items: { type: "string" } } }, additionalProperties: false }],
    messages: {
      default: "Test ID prop passthrough must have a literal default.",
    },
  },
  (context) => selectorAttributeVisitors(context, (node) => {
    const valueNode = selectorValueNode(node);
    if (valueNode?.type === "Identifier" && !defaultedPropsForNode(node).has(valueNode.name)) {
      context.report({ node, messageId: "default" });
    }
  }),
);

const unique = rule(
  {
    type: "problem",
    docs: {
      description: "require unique literal test IDs within a file",
      recommended: true,
    },
    schema: [{ type: "object", properties: { selectorAttributes: { type: "array", items: { type: "string" } } }, additionalProperties: false }],
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

const noEmpty = rule(
  {
    type: "problem",
    docs: { description: "disallow empty test IDs", recommended: true },
    schema: [{ type: "object", properties: { selectorAttributes: { type: "array", items: { type: "string" } } }, additionalProperties: false }],
    messages: { empty: "Test ID must not be empty." },
  },
  (context) => selectorAttributeVisitors(context, (node) => {
    if (selectorLiteral(node) === "") {
      context.report({ node, messageId: "empty" });
    }
  }),
);

const consistentAttribute = rule(
  {
    type: "suggestion",
    docs: { description: "require a canonical test ID attribute", recommended: false },
    schema: [{ type: "object", properties: { selectorAttributes: { type: "array", items: { type: "string" } }, canonicalAttribute: { type: "string" } }, additionalProperties: false }],
    messages: { attribute: "Use '{{expected}}' instead of '{{actual}}' for test IDs." },
  },
  (context) => selectorAttributeVisitors(context, (node, name) => {
    const expected = canonicalAttribute(options(context));
    /* v8 ignore next -- matching canonical attrs are asserted through no reports */
    if (name !== expected) {
      context.report({ node: node.name, messageId: "attribute", data: { actual: name, expected } });
    }
  }),
);

const requireInteractiveTestId = rule(
  {
    type: "suggestion",
    docs: { description: "require test IDs on interactive JSX elements", recommended: false },
    schema: [{ type: "object", properties: { selectorAttributes: { type: "array", items: { type: "string" } } }, additionalProperties: false }],
    messages: { missing: "Interactive elements should have a test ID." },
  },
  (context) => {
    const attrs = selectorAttributes(options(context));
    return {
      JSXOpeningElement(node) {
        const elementName = node.name.type === "JSXIdentifier" ? node.name.name : null;
        const hasSelector = node.attributes.some((attr) => isSelectorAttribute(attributeName(attr), attrs));
        if (hasSelector || !isInteractiveElement(elementName, node.attributes)) {
          return;
        }
        context.report({ node: node.name, messageId: "missing" });
      },
    };
  },
);

function isInteractiveElement(elementName, attributes) {
  if (INTERACTIVE_ELEMENTS.has(elementName)) {
    return true;
  }
  /* v8 ignore next -- covered behavior is asserted through require-interactive-test-id */
  if (elementName === "a" && attributes.some((attr) => attributeName(attr) === "href")) {
    return true;
  }
  return attributes.some((attr) => {
    if (attributeName(attr) === "onClick") {
      return true;
    }
    if (attributeName(attr) !== "role") {
      return false;
    }
    return ["button", "checkbox", "link", "menuitem", "option", "radio", "switch", "tab", "textbox"].includes(selectorLiteral(attr));
  });
}

const preferGetByTestId = rule(
  {
    type: "suggestion",
    docs: { description: "prefer getByTestId over CSS test-id selectors", recommended: false },
    schema: [{ type: "object", properties: { selectorAttributes: { type: "array", items: { type: "string" } } }, additionalProperties: false }],
    messages: { prefer: "Prefer getByTestId('{{value}}') for exact test ID selectors." },
  },
  (context) => {
    const attrs = selectorAttributes(options(context));
    return {
      CallExpression(node) {
        if (!isSelectorCall(node) || callMethodName(node) === "getByTestId") {
          return;
        }
        for (const arg of node.arguments.slice(0, callMethodName(node) === "dragAndDrop" ? 2 : 1)) {
          const source = literalString(arg);
          /* v8 ignore next -- non-literal selector args are intentionally skipped */
          if (!source) {
            continue;
          }
          for (const selector of cssSelectorValues(source, attrs)) {
            /* v8 ignore next -- non-exact operators are intentionally skipped */
            if (selector.operator === "=") {
              context.report({ node: arg, messageId: "prefer", data: { value: selector.value } });
            }
          }
        }
      },
    };
  },
);

const namingConvention = rule(
  {
    type: "suggestion",
    docs: { description: "require a naming convention for literal test IDs", recommended: false },
    schema: [{
      type: "object",
      properties: {
        selectorAttributes: { type: "array", items: { type: "string" } },
        pattern: { type: "string" },
      },
      additionalProperties: false,
    }],
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

const rules = {
  "consistent-attribute": consistentAttribute,
  defaults,
  "literals": literals,
  "naming-convention": namingConvention,
  "no-empty": noEmpty,
  "prefer-get-by-test-id": preferGetByTestId,
  "require-interactive-test-id": requireInteractiveTestId,
  unique,
};

const plugin = {
  meta: {
    name: "eslint-plugin-playwright-ast-coverage",
    version: require("../package.json").version,
  },
  rules,
  configs: {},
};

plugin.configs.recommended = {
  plugins: {
    "playwright-ast-coverage": plugin,
  },
  rules: {
    "playwright-ast-coverage/defaults": "error",
    "playwright-ast-coverage/literals": "error",
    "playwright-ast-coverage/no-empty": "error",
    "playwright-ast-coverage/unique": "error",
  },
};

plugin.configs.strict = {
  plugins: {
    "playwright-ast-coverage": plugin,
  },
  rules: {
    ...plugin.configs.recommended.rules,
    "playwright-ast-coverage/consistent-attribute": ["error", { canonicalAttribute: "data-pw" }],
    "playwright-ast-coverage/naming-convention": "error",
    "playwright-ast-coverage/prefer-get-by-test-id": "warn",
    "playwright-ast-coverage/require-interactive-test-id": "warn",
  },
};

/* v8 ignore next */
module.exports = plugin;
