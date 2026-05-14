export default {
  name: 'storybook',
  testDir: './playwright/storybook',
  use: {
    baseURL: 'http://localhost:6006',
    testIdAttribute: 'data-pw',
  },
  projects: [
    { name: 'chromium' },
  ],
};
