await navigateTo(page, '/settings');
await testHelpers.openPath(page, "/profile");
await notnavigateTo(page, '/ignored-prefix');
await navigateToSomething(page, '/ignored-suffix');
