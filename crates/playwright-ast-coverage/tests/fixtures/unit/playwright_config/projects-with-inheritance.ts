export default defineConfig({
  testDir: './tests',
  testIgnore: ['**/skip/**'],
  use: { baseURL: 'http://localhost:3000', testIdAttribute: 'data-pw' },
  projects: [
    { name: 'chromium', testMatch: '**/*.spec.ts' },
    { name: 'webkit', testDir: './e2e', testMatch: ['**/*.pw.ts'], use: { testIdAttribute: 'data-test' } },
  ],
});
