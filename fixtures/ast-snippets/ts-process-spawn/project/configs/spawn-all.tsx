const ready = true;

exec("node scripts/root.mts");
exec("npm run scripts/root.mts");
exec("node missing.mts");
exec("node missing-again.mts");
execFile("scripts/exec-file.mts");
fork("scripts/fork.mts");
spawn("scripts/direct-spawn.mts");
spawn("scripts/direct-spawn.mts", [], "not-options");
spawn("scripts/spawn.mts", [], { cwd: "apps/site" });
spawn("scripts/spawn.mts", [], { ...spawnOptions, cwd: "apps/site" });
spawn("scripts/no-cwd-match.mts", [], { cwd: dynamicCwd });
spawn("scripts/no-cwd-property.mts", [], { env: {} });

const config = {
  webServer: [
    { command: "bun scripts/web.mts", cwd: "apps/site" },
    { command: `node scripts/template-web.mts`, cwd: "apps/site" },
    { command: "node scripts/missing-web.mts", cwd: "apps/site" },
    { command: runDynamic, cwd: "apps/site" },
    { ["command"]: "node scripts/ignored-computed-key.mts" },
  ],
  nested: { command: "node scripts/object-nested.mts" },
};

defineConfig({
  ...sharedConfig,
  webServer: { command: "tsx scripts/define-config.mts" },
});

defineConfig({ other: true });

export default defineConfig({
  webServer: { command: "tsx scripts/export-default-config.mts" },
});

export default defineConfig();

{
  exec("tsx scripts/block.mts");
}

function declared() {
  return exec("node scripts/function.mts");
}

export const exported = exec("node scripts/export-var.mts");

export function exportedFunction() {
  exec("node scripts/export-function.mts");
}

export default function defaultFunction() {
  exec("node scripts/default-function.mts");
}

export default () => {
  exec("node scripts/default-arrow.mts");
};

if (ready) {
  exec("node scripts/if.mts");
} else {
  exec("node scripts/else.mts");
}

try {
  exec("node scripts/try.mts");
} catch (error) {
  exec("node scripts/catch.mts");
} finally {
  exec("node scripts/finally.mts");
}

while (ready) {
  exec("node scripts/while.mts");
  break;
}

for (let i = 0; i < 1; i++) {
  exec("node scripts/for.mts");
}

for (const key in items) {
  exec("node scripts/for-in.mts");
}

for (const item of items) {
  exec("node scripts/for-of.mts");
}

const asyncRunner = async () => {
  await exec("node scripts/await.mts");
};

const memberRunner = childProcess.exec("node scripts/member-exec.mts");
const unknownCallee = makeRunner()("node scripts/ignored-unknown-callee.mts");
const ignoredDynamic = exec(command);
const ignoredHttp = exec("node http://example.com/script.mts");
const nested = wrap(exec("node scripts/nested.mts"));
