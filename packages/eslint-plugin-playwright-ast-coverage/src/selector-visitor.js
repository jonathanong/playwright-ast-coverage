"use strict";

const { attributeName, isSelectorAttribute, options, selectorAttributes } = require("./helpers");

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

module.exports = {
  selectorAttributeVisitors,
};
