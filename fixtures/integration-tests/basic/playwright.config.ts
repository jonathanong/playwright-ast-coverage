import { defineConfig } from '@playwright/test'

export default defineConfig({
  projects: [
    {
      name: 'pw-unit',
      testDir: './playwright/unit',
      testMatch: ['**/*.spec.ts'],
    },
    {
      name: 'pw-openai',
      testDir: './playwright/openai',
      testMatch: ['**/*.spec.ts'],
    },
  ],
})
