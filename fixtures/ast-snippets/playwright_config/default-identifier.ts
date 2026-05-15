const config = {
  testDir: './identifier-tests',
  testMatch: '**/*.identifier.ts',
  use: {
    baseURL: 'http://localhost:4100',
    testIdAttribute: 'data-identifier',
  },
};

export default config;
