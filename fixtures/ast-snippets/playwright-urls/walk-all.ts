const setup = page.goto("/var-init");
page.goto(("/expr-string"));
page.goto((`/expr-template`));
let i = 0;

declare function ambient(): void;

function helper() {
  return page.goto("/return");
}

{
  page.goto("/block");
}

if (page.goto("/if-test")) {
  page.goto("/if");
} else {
  page.goto("/else");
}

try {
  page.goto("/try");
} catch {
  page.goto("/catch");
} finally {
  page.goto("/finally");
}

while (page.goto("/while-test")) {
  page.goto("/while-body");
  break;
}

for (page.goto("/for-init-expr"); page.goto("/for-test"); page.goto("/for-update")) {
  page.goto("/for-body");
}

for (let j = page.goto("/for-var-init"); j < 1; j++) {
  page.goto("/for-var-body");
}

for (;;) {
  page.goto("/for-empty");
  break;
}

do {
  page.goto("/do-body");
} while (page.goto("/do-test"));

for (const key in page.goto("/for-in-right")) {
  page.goto("/for-in-body");
}

for (const value of page.goto("/for-of-right")) {
  page.goto("/for-of-body");
}

const arrow = () => {
  page.goto("/arrow");
};

const conditional = ready ? page.goto("/conditional") : page.goto("/alternate");
const logical = ready && page.goto("/logical");
page.goto("/sequence-one"), page.goto("/sequence-two");

switch (page.goto("/switch-discriminant")) {
  case page.goto("/switch-case"):
    page.goto("/switch-body");
    break;
}

expect(other).toHaveURL("/ignored-other-expect");
assert(page).toHaveURL("/ignored-non-expect");
namespace.expect(page).toHaveURL("/ignored-member-expect");
expect(page).toHaveURL(new URL("/ignored-url", base));
expect(page).toHaveURL(new RegExp(dynamic));
expect(page).toHaveURL(new namespace.RegExp("/ignored-member-regexp"));
navigateTo(other, "/navigate-second");
navigateTo(page, "relative");
page.waitForURL("relative");
page.click("a[href=/missing-quote]");
