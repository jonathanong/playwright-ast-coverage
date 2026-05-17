import { defineConfig } from '@playwright/test'

export default defineConfig(({
  name: 'root',
  testDir: './root',
  testIgnore: '**/root-ignore.ts',
  projects: [
    {
      name: `absolute`,
      testDir: '/tmp/no-mistakes-absolute-tests',
      testMatch: [`**/*.spec.ts`],
      testIgnore: '**/skip.ts',
    },
    {
      name: 'inherits',
      testMatch: ['**/*.test.ts'],
    },
    {
      ['name']: 'computed',
      method() {},
    },
    1,
  ],
}))
