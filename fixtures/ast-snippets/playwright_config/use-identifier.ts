const sharedUse = {
  baseURL: 'http://localhost:6200',
  testIdAttribute: 'data-shared',
};

export default defineConfig({
  testDir: './use-identifier-tests',
  use: sharedUse,
});
