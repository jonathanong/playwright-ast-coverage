page.locator('[data-testid="user-123-button"]');
page.locator('[data-testid^="user-"]');
page.locator('[data-testid$="-button"]');
page.locator('[data-testid*="user-"]');
page.getByTestId(/^user-/);
