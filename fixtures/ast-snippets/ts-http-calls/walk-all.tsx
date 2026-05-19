const top = client.get("/api/top");
const ignored = client.get("/other/top");

export const fromVar = client.post("/api/var");

export function exportedFunction() {
  return client.put("/api/exported-function");
}

export default () => {
  client.patch("/api/default-arrow");
};

export default function namedDefault() {
  client.delete("/api/default-function");
}

if (client.get("/api/if-test")) {
  client.head("/api/if");
} else {
  client.options("/api/else");
}

try {
  fetch("/api/try");
} catch (error) {
  fetch("/api/catch");
} finally {
  fetch("/api/finally");
}

for (let item = fetch("/api/for-init"); fetch("/api/for-test"); fetch("/api/for-update")) {
  fetch("/api/for-body");
}

for (const key in fetch("/api/for-in-right")) {
  fetch("/api/for-in");
}

for (const value of fetch("/api/for-of-right")) {
  fetch("/api/for-of");
}

while (fetch("/api/while-test")) {
  fetch("/api/while");
}

do {
  fetch("/api/do-while");
} while (fetch("/api/do-while-test"));

const arrow = () => {
  fetch("/api/arrow");
};

const conditional = fetch("/api/conditional-test") ? fetch("/api/conditional") : fetch("/api/alternate");
const logical = ready && fetch("/api/logical");
const sequence = (fetch("/api/sequence-one"), fetch("/api/sequence-two"));
const chained = client.wrap().get("/api/chained");
const casted = fetch("/api/casted") as unknown;
const nonNull = fetch("/api/non-null")!;
