const handler = () => {};
const ready = true;

declare function declaredOnly(): void;

app.route("/chain").get(handler).post(handler);
app.put("/direct", handler);
wrap(app.patch("/wrapped", handler), other("/ignored"));

{
  app.delete("/block", handler);
}

function nested() {
  if (ready) {
    app.head("/if", handler);
  } else {
    app.options("/else", handler);
  }
  while (ready) {
    break;
  }
  do {
    break;
  } while (ready);
  switch (ready) {
    case true:
      app.get("/switch", handler);
      break;
    default:
      app.get("/default", handler);
  }
  try {
    app.get("/try", handler);
  } catch (error) {
    app.get("/catch", handler);
  } finally {
    app.get("/finally", handler);
  }
}

export const exportedRoute = app.get("/export-var", handler);

export function exportedFunction() {
  app.get("/export-function", handler);
}

export function exportedShadowedFunction(app) {
  app.get("/ignored-export-param", handler);
}

export const exportedShadowedVar = (() => {
  const app = fake;
  app.get("/ignored-export-var", handler);
})();

export class ExportedClass {}

function shadowedBlocks() {
  const app = fake;
  app.get("/ignored-const", handler);
}

function shadowedPatterns() {
  const { app: alias, other } = fake;
  const [app] = fake;
  const [first, ...rest] = fake;
  const { nested: { app: nestedApp }, ...others } = fake;
  alias.get("/ignored-alias", handler);
  nestedApp.get("/ignored-nested", handler);
}

function shadowedObjectRestPattern() {
  const { ...app } = fakeObject;
  app.get("/ignored-object-rest", handler);
}

function shadowedArrayRestPattern() {
  const [...app] = fakeArray;
  app.get("/ignored-array-rest", handler);
}

function shadowedParamPatterns({ app }, [other], ...rest) {
  app.get("/ignored-param-pattern", handler);
}

function shadowedDefaultParam(app = fake) {
  app.get("/ignored-default-param", handler);
}

function shadowedRestParam(...app) {
  app.get("/ignored-rest-param", handler);
}

function shadowedVarInControlFlow() {
  if (ready) {
    var app = fake;
  }
  app.get("/ignored-var-if", handler);
}

function shadowedForIn() {
  for (var app in apps) {
  }
  app.get("/ignored-for-in", handler);
}

function shadowedForOf() {
  for (var app of apps) {
  }
  app.get("/ignored-for-of", handler);
}

function shadowedForInBody() {
  for (const key in apps) {
    var app = fake;
  }
  app.get("/ignored-for-in-body", handler);
}

function shadowedForOfBody() {
  for (const item of apps) {
    var app = fake;
  }
  app.get("/ignored-for-of-body", handler);
}

function shadowedAssignmentPattern({ value: app = fake }) {
  app.get("/ignored-assignment-pattern", handler);
}

function shadowedClass() {
  class app {}
  app.get("/ignored-class", handler);
}

function shadowedNestedFunction() {
  function app() {}
  app.get("/ignored-nested-function", handler);
}

function app() {
  app.get("/ignored-function-name", handler);
}

function shadowedInLoopsAndTry() {
  for (let i = 0; i < 1; i++) {
    var app = fake;
  }
  while (ready) {
    var whileOnly = fake;
    break;
  }
  do {
    var doOnly = fake;
    break;
  } while (ready);
  switch (ready) {
    case true:
      var switchOnly = fake;
      break;
  }
  try {
    var tryOnly = fake;
  } catch {
    var catchOnly = fake;
  } finally {
    var app = fake;
  }
  app.get("/ignored-control-flow-var", handler);
}

function nonMatchingRouteShapes() {
  app.get(handler);
  other.route("/ignored-other").get(handler);
  other.wrapper().route("/ignored-nested-other").get(handler);
  other.get("/ignored-direct-other", handler);
  routeFactory().get("/ignored-factory", handler);
  app.route(`/ignored-${dynamic}`).get(handler);
}
