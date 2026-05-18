"use strict";

const { copyFile, readFile } = require("node:fs/promises");
const { createWriteStream } = require("node:fs");
const http = require("node:http");
const https = require("node:https");
const { pipeline } = require("node:stream/promises");
const { fileURLToPath } = require("node:url");

const DOWNLOAD_TIMEOUT_MS = 30_000;

function download(url, destination, redirects = 0) {
  if (url.startsWith("file://")) {
    return copyFile(fileURLToPath(url), destination);
  }

  return request(
    url,
    async (response) => {
      await pipeline(response, createWriteStream(destination));
    },
    redirects,
  );
}

function request(
  url,
  handleResponse,
  redirects = 0,
  clients = { http, https },
  timeoutMs = DOWNLOAD_TIMEOUT_MS,
) {
  return new Promise((resolve, reject) => {
    const client = url.startsWith("http://") ? clients.http : clients.https;
    const req = client.get(url, (response) => {
      if (isRedirectStatus(response.statusCode)) {
        response.resume();
        if (redirects >= 5) {
          reject(new Error(`Too many redirects while downloading ${url}`));
          return;
        }
        if (!response.headers.location) {
          reject(new Error(`Redirect missing Location header while downloading ${url}`));
          return;
        }
        request(
          new URL(response.headers.location, url).toString(),
          handleResponse,
          redirects + 1,
          clients,
          timeoutMs,
        ).then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`Download failed for ${url}: HTTP ${response.statusCode}`));
        return;
      }

      Promise.resolve(handleResponse(response)).then(resolve, reject);
    });

    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`Download timed out after ${timeoutMs}ms: ${url}`));
    });
    req.on("error", reject);
  });
}

function isRedirectStatus(statusCode) {
  return [301, 302, 303, 307, 308].includes(statusCode);
}

async function fetchText(url) {
  if (url.startsWith("file://")) {
    return readFile(fileURLToPath(url), "utf8");
  }
  const chunks = [];
  let totalLength = 0;
  const MAX_LENGTH = 1024 * 1024; // 1MB limit to prevent DoS memory exhaustion
  await request(url, async (response) => {
    for await (const chunk of response) {
      totalLength += chunk.length;
      if (totalLength > MAX_LENGTH) {
        throw new Error(`Response exceeded maximum size of ${MAX_LENGTH} bytes`);
      }
      chunks.push(chunk);
    }
  });
  return Buffer.concat(chunks).toString("utf8");
}

module.exports = {
  download,
  fetchText,
  isRedirectStatus,
  request,
};
