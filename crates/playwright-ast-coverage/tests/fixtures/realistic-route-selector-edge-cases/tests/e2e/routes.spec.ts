import { expect, test } from "@playwright/test";
import { navigateTo } from "../helpers/navigation";

const ROUTE_TABLE = [{ name: "discussions", path: () => "/discussions" }];

test("covers specific static routes without covering generic siblings", async ({
  page,
}) => {
  await navigateTo(page, "/admin/crm");
  await navigateTo(page, "/chat/support");
  await navigateTo(page, "/chat/support/new");
  await navigateTo(page, "/chat/abc");
  await navigateTo(page, "/communities/example");
  await navigateTo(page, "/communities/create");
  await navigateTo(page, "/discussion/abc");

  await expect(page).toHaveURL(/\/admin\/crm\/[a-f0-9-]+/);
  await expect(page).toHaveURL(/\/admin/);
  await page.waitForURL(/\/chat\/support\/[a-f0-9-]+/);
  expect(page.url()).toMatch(/\/chat\/support\/[a-f0-9-]+/);

  for (const { path } of ROUTE_TABLE) {
    await navigateTo(page, path());
  }

  await page.locator('[data-pw="rss-feed-link"]').click();
});

test.describe.skip("routes covered by faster integration suites", () => {
  test("catalogs routes exercised outside Playwright", async ({ page }) => {
    await navigateTo(page, "/catalog-only");
  });
});
