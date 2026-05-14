const root = { baseURL: 'ignored' };

export default defineConfig({
  ...root,
  ['computed']: 'ignored',
  helper() {
    return null;
  },
  123: 'ignored',
  "testDir": `./tests`,
  testMatch: ([`**/*.spec.ts`, /ignored/]),
  testIgnore: [ignored, '**/skip/**'],
  use: ({ baseURL: `http://localhost:3000`, testIdAttribute: 'data-test' }),
  projects: [
    ignored,
    {
      testDir: ('./project-tests'),
      testMatch: ('**/*.project.ts'),
      use: makeUse(),
    },
  ],
});
