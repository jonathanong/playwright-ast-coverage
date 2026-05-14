export default {
  name: 'web',
  testDir: './playwright/tests',
  use: {
    baseURL: 'http://localhost:3000',
    testIdAttribute: 'data-pw',
  },
  projects: [
    { name: 'chromium' },
  ],
};
