const BASE_URL = process.env.BASE_URL ?? 'http://localhost:3000';

export default {
  testDir: './tests/e2e',
  testMatch: '**/*.spec.ts',
  use: {
    baseURL: BASE_URL,
  },
};
