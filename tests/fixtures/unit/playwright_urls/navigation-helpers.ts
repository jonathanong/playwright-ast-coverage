await navigateTo(page, '/settings');
await testHelpers.openPath(page, "/profile");
await helpers.navigateTo(page, '/team');
await getNavigateTo()(page, '/ignored-dynamic');
await notnavigateTo(page, '/ignored-prefix');
await navigateToSomething(page, '/ignored-suffix');
