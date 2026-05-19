const assert = require("node:assert/strict");
const { createServer } = require("node:http");
const { EventEmitter } = require("node:events");
const { mkdtemp, readFile, rm, writeFile } = require("node:fs/promises");
const { join } = require("node:path");
const { tmpdir } = require("node:os");
const { pathToFileURL } = require("node:url");

const { download, fetchText, isRedirectStatus } = require("./install");
const { request } = require("no-mistakes-core");

test("downloads file URLs and fetches text over HTTP redirects", async () => {
  const root = await mkdtemp(join(tmpdir(), "playwright-ast-coverage-download-"));
  const source = join(root, "source.txt");
  const destination = join(root, "destination.txt");
  await writeFile(source, "hello");

  const server = createServer((request, response) => {
    if (request.url === "/missing-location") {
      response.writeHead(302);
      response.end();
      return;
    }
    if (request.url === "/redirect") {
      response.writeHead(302, { location: "/text" });
      response.end();
      return;
    }
    if (request.url === "/text") {
      response.writeHead(200);
      response.end("redirected");
      return;
    }
    response.writeHead(404);
    response.end("not found");
  });

  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();

  try {
    await download(pathToFileURL(source).toString(), destination);
    assert.equal(await readFile(destination, "utf8"), "hello");
    assert.equal(await fetchText(`http://127.0.0.1:${address.port}/redirect`), "redirected");
    await assert.rejects(() => fetchText(`http://127.0.0.1:${address.port}/missing`), /HTTP 404/);
    await assert.rejects(
      () => fetchText(`http://127.0.0.1:${address.port}/missing-location`),
      /missing Location/i,
    );
    await assert.rejects(() => fetchText(`https://127.0.0.1:${address.port}/text`));
  } finally {
    await new Promise((resolve) => server.close(resolve));
    await rm(root, { recursive: true, force: true });
  }
});

test("classifies redirect status codes", () => {
  assert.equal(isRedirectStatus(301), true);
  assert.equal(isRedirectStatus(undefined), false);
});

test("rejects redirect loops", async () => {
  const server = createServer((_request, response) => {
    response.writeHead(302, { location: "/loop" });
    response.end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();

  try {
    await assert.rejects(
      () => fetchText(`http://127.0.0.1:${address.port}/loop`),
      /Too many redirects/,
    );
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});

test("request rejects timeout errors", async () => {
  const client = {
    get() {
      const req = new EventEmitter();
      req.setTimeout = (_timeout, callback) => {
        queueMicrotask(callback);
      };
      req.destroy = (error) => {
        req.emit("error", error);
      };
      return req;
    },
  };

  await assert.rejects(
    () => request("http://example.test/file", () => {}, 0, { http: client, https: client }, 1),
    /timed out after 1ms/,
  );
});
