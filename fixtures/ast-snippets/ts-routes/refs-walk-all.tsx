import "next/navigation";
import navDefault, * as navAll from "next/navigation";
import { redirect as go, notRedirect } from "next/navigation";
import { redirect as unrelated } from "other/navigation";

declare function Declared(): void;
export declare function ExportedDeclared(): void;

const router = useRouter();
const { push, replace: swap = fallback } = useRouter();
const { [dynamicMethod]: dynamicMethod } = useRouter();
const memberRouter = navAll.useRouter();

router.push("/router");
router.replace({ pathname: "/router/[id]" });
router.prefetch(`/prefetch/${slug}`);
memberRouter.push("/member-router");
push("/method");
swap("/replace");
go("/redirect");
fetch("/api/local");
api.fetch("/api/member");
helper(...router.push("/spread"));

const link = (
  <>
    <a href="/href">Href</a>
    <Link to={{ pathname: `/to/${slug}` }}>To</Link>
    <Link href={{ "pathname": "/string-key/[slug]/" }}>String key</Link>
    <Link href={{ pathname: "/catch-all/[...slug]" }}>Catch all</Link>
    <Link href={{ pathname: "/optional/[[...slug]]" }}>Optional catch all</Link>
    <Link href={{ ...routeObj, pathname: "/after-spread" }}>After spread</Link>
    <Link href={{ [pathnameKey]: "/ignored-computed-key" }}>Computed key</Link>
    <Link other="/ignored-other-attr">Other attr</Link>
    <Link href={dynamicHref}>Dynamic href</Link>
    <Link href=<span /> />
    <Link href=<></> />
    <Link href>Boolean href</Link>
    <Link href="?query">Skipped</Link>
    <ns:Link href="/namespaced">Namespaced</ns:Link>
    <Link ns:other="/ignored-namespaced-attr">Namespaced attr</Link>
    <Link {...props} href="/spread-attr">Spread attr</Link>
    <>
      <Link href="/nested-fragment">Nested fragment</Link>
    </>
    {/* empty */}
    {ready && <Link href={"/expr" as string}>Expr</Link>}
    {ready && <Link href={<span />}>Expression with JSX child</Link>}
    {...props}
  </>
);

function Component({ router: localRouter, push: localPush, ...rest }) {
  if (ready) {
    router.push("/if");
  } else {
    router.push("/else");
  }
  const value = ready ? router.push("/conditional") : push("/alternate");
  const seq = (router.push("/sequence-one"), router.push("/sequence-two"));
  const assigned = (target = router.push("/assignment"));
  const asserted = (router.push("/assertion-call") as unknown);
  const satisfies = router.push("/satisfies") satisfies unknown;
  const nonNull = router.push("/non-null")!;
  const parenthesized = (router.push("/parenthesized"));
  return <a href="/return">Return</a>;
}

function Shadowing() {
  switch (kind) {
    case "one":
      var go = local;
      break;
    default:
      break;
  }
  try {
    var push = localPush;
  } catch (error) {
    var router = localRouter;
  } finally {
    var swap = localSwap;
  }
  go("/ignored-switch-try-var");
  push("/ignored-switch-try-push");
  router.push("/ignored-switch-try-router");
  swap("/ignored-switch-try-swap");
}

const Handler = () => {
  const router = useRouter();
  while (ready) {
    var go = local;
    break;
  }
  go("/ignored-shadowed-var");
  router.push("/arrow");
};

const FnExpr = function go([push = fallback, ...rest]) {
  go("/ignored-function-name");
  push("/ignored-param");
  return router.push("/function-expression");
};

export const Exported = () => push("/export-var");
export const ExportedInit = router.push("/export-var-init");

export function NamedExport() {
  swap("/export-function");
}

export class redirect {}
export enum OtherExport {
  A,
}

export default router.push("/default-expression");

export class push {}

function DefaultExport({ go }) {
  go("/ignored-param-redirect");
  return router.push("/default-export");
}

DefaultExport({});

for (var push of handlers) {
}
push("/ignored-for-of-var");

for (var router = useRouter(); ready; ready = false) {
}
router.push("/for-var-router");

for (var topPush = useRouter().push; ready; ready = false) {
}

for (let localRouter = useRouter(); ready; ready = false) {
}

for (let keep of handlers) {
}
swap("/after-let-for-of");

for (router of handlers) {
}

class LocalRouter {
}
router.push("/after-class-shadow-check");

export function ExportedFunctionWithBody({ replace }) {
  const router = useRouter();
  replace("/ignored-export-param");
  return router.replace("/exported-function-body");
}

export const ExportedNested = (() => {
  return router.push("/exported-nested-expression");
})();

export default function DefaultFunction({ prefetch = fallback }) {
  const nav = useRouter();
  if (ready) {
    prefetch("/ignored-default-param");
  }
  return nav.prefetch({ pathname: "/default-function-body/[id]" });
}

function MoreVarScopes() {
  const { replace } = useRouter();
  for (var replace in handlers) {
  }
  replace("/ignored-for-in-var");
}

function MoreFunctionScopeBranches() {
  if (ready) {
    var prefetch = localPrefetch;
  }
  while (ready) {
    var otherLocal = localPrefetch;
    break;
  }
  do {
    var anotherLocal = localPrefetch;
    break;
  } while (ready);
  switch (kind) {
    case "prefetch":
      var yetAnotherLocal = localPrefetch;
      break;
  }
  try {
    var tryLocal = localPrefetch;
  } catch (error) {
    var catchLocal = localPrefetch;
  } finally {
    var finalLocal = localPrefetch;
  }
  prefetch("/ignored-function-scope-shadow");
}

function DoWhileScope() {
  const { prefetch } = useRouter();
  do {
    var local = localPrefetch;
  } while (ready);
  prefetch("/do-while-after");
}

const [arrayRouter] = useRouter();
const { push: { nestedPush } } = useRouter();
notRouter.push("/ignored-unbound");
push();
router.push("https://example.com/ignored-router");
router.push(...args);
other.push("/ignored-other-member");
getRouter().push("/ignored-call-object");
router.back("/ignored-back");
dynamicCall("/ignored-dynamic");
fetch();
fetch("https://example.com/ignored");
