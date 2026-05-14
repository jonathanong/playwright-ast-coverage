await expect(page).toHaveURL('/settings');
await expect(page).toHaveURL(new RegExp(`/user/${username}/rss-feed-items/viewed`));
