export default {
  projects: [
    {
      name: 'web',
      testDir: './tests/web',
      testMatch: '**/*.spec.ts',
      use: {
        baseURL: 'http://localhost:3000',
        testIdAttribute: 'data-pw',
      },
    },
    {
      name: 'storybook',
      testDir: './tests/storybook',
      testMatch: '**/*.spec.ts',
      use: {
        baseURL: 'http://localhost:6006',
        testIdAttribute: 'data-testid',
      },
    },
  ],
};
