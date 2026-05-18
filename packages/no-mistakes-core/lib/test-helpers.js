"use strict";

const { EventEmitter } = require("node:events");

function _runWithChild(run, runArgs, event, ...eventArgs) {
  const child = new EventEmitter();
  const exits = [];
  const spawnCalls = [];
  run(...runArgs, { exit: (code) => exits.push(code) }, (bin, argv, options) => {
    spawnCalls.push([bin, argv, options]);
    queueMicrotask(() => child.emit(event, ...eventArgs));
    return child;
  });
  return new Promise((resolve) => {
    setImmediate(() => resolve({ exits, spawnCalls }));
  });
}

function runWithChild(run, defaultArgs, event, ...eventArgs) {
  return _runWithChild(run, [defaultArgs, "linux"], event, ...eventArgs);
}

function runWithChildWithEnv(run, defaultArgs, event, ...eventArgs) {
  return _runWithChild(run, [defaultArgs, {}, "linux"], event, ...eventArgs);
}

async function testInstallerFailures(main, assert) {
  const exits = [];
  const errors = [];

  const logger = () => {};
  await main(
    async () => {
      throw new Error("install failed");
    },
    { exit: (code) => exits.push(code) },
    { log: logger, error: (message) => errors.push(message) },
  );
  assert.deepEqual(exits, [1]);
  assert.deepEqual(errors, ["install failed"]);
  await main(
    async () => {
      throw "string failed";
    },
    { exit: (code) => exits.push(code) },
    { log: logger, error: (message) => errors.push(message) },
  );
  assert.deepEqual(exits, [1, 1]);
  assert.deepEqual(errors, ["install failed", "string failed"]);
}

module.exports = {
  runWithChild,
  runWithChildWithEnv,
  testInstallerFailures,
};
